//! The factored answers against the scenario corpus (docs/ai-player-plan.md, "Answers"). At every request a scenario
//! answers, the answer given is put in canonical terms (`Answering::factored`), decomposed into tokens
//! (`cantrip_obs::factor::decompose`), replayed through the step machine - which must make the same answer - and
//! turned back into the engine's, which the engine's validator must accept and which must be the answer given (up to
//! the order of a list, and payment, made by `pay::pay` in the factoring's terms). Then a few seeded random walks
//! through the machine (deliberate fuzzing, not a search for a state) each make an answer that the validator must
//! accept, or refuse only for a property of the whole answer the factoring leaves to it (`Rejection::global`).
//!
//! The fast test takes a sample of the corpus; `cargo test -p cantrip-ai --test factoring -- --ignored` takes all of
//! it. Both print each kind's walks accepted and refused, and the kinds no scenario raised.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cantrip_ai::{Answering, Knowledge, factored_name, final_x, observe_indexed};
use cantrip_bench::Random;
use cantrip_engine::*;
use cantrip_obs::factor::{FactoredAnswer, Machine, X_BUCKETS, kind_name};
use cantrip_obs::{AskKind, InfoMode};
use cantrip_script::GameScript;

const KINDS: [&str; 26] = [
    "Priority",
    "PayCosts",
    "UnlessPays",
    "ChooseCards",
    "ChooseTarget",
    "ChooseModes",
    "ChooseColors",
    "ChooseName",
    "ChooseSubtype",
    "May",
    "ChoosePlayer",
    "TapForPower",
    "CollectEvidence",
    "Craft",
    "RemoveCounters",
    "ChooseDividedTargets",
    "DeclareAttackers",
    "AssignCombatDamage",
    "DeclareBlockers",
    "DiscardToHandSize",
    "Mulligan",
    "ChooseStartingPlayer",
    "OrderTriggers",
    "DistributeCards",
    "SearchLibrary",
    "ChooseReplacement",
];

const WALKS: u64 = 3;

#[derive(Default)]
struct Tally {
    requests: BTreeMap<String, usize>,
    accepted: BTreeMap<String, usize>,
    global: BTreeMap<String, usize>,
    local: BTreeMap<String, Vec<String>>,
    unfactored: usize,
    clamped: usize,
    trips: usize,
    failures: Vec<String>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scenarios() -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(root().join("scenarios"))
        .expect("scenarios")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "cantrip"))
        .collect();
    all.sort();
    all
}

/// Lists in the order the factoring makes them in, for comparison: what is a set to the engine, sorted.
fn normal(a: &Answer) -> Answer {
    let mut a = a.clone();
    match &mut a {
        Answer::Act(act) if act.kind == ActionKind::CancelCasting => return Answer::Cancel,
        Answer::Cards(c) => c.sort(),
        Answer::Attack(x) => x.sort_by_key(|&(c, _)| c),
        Answer::Block(x) => x.sort(),
        Answer::Shares(x) => x.sort(),
        Answer::Divided(x) => x.sort_by_key(|&(t, _)| format!("{t:?}")),
        Answer::Counters(x) => x.sort_by_key(|&(o, k, _)| (o, k as u16)),
        Answer::Placements { placements, .. } => placements.iter_mut().for_each(|p| p.sort()),
        Answer::Modes(m) => m.sort(),
        Answer::Colors(c) => c.sort_by_key(|&c| c as u8),
        _ => {}
    }
    a
}

/// A factored answer with what is a set to the engine sorted: the machine makes those lists in its own order.
fn canon(f: &FactoredAnswer) -> FactoredAnswer {
    use FactoredAnswer as F;
    let mut f = f.clone();
    match &mut f {
        F::Attack(x) | F::Block(x) => x.sort(),
        F::Divided(x) => x.sort(),
        F::Counters(x) => x.sort(),
        F::Shares(x) => x.sort(),
        F::Modes(x) | F::Colors(x) => x.sort(),
        F::Placements { placements, .. } => placements.iter_mut().for_each(|p| p.sort()),
        _ => {}
    }
    f
}

fn payment(f: &FactoredAnswer) -> bool {
    matches!(
        f,
        FactoredAnswer::AutoPay | FactoredAnswer::X(_) | FactoredAnswer::Unless { pay: true, .. }
    )
}

fn play(path: &Path, tally: &Mutex<Tally>) {
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let Ok(script) = GameScript::load(path) else {
        return;
    };
    let Ok(result) = cantrip_script::run(&script) else {
        return;
    };
    let mut game = result.game;
    let nodes = game.tree.path(game.at);
    let answers: Vec<_> = nodes
        .iter()
        .filter_map(|&n| game.tree.answer(n).map(|(_, p, a)| (n, *p, a.clone())))
        .collect();
    let mut random = Random::new(
        name.bytes()
            .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64)),
    );
    for (k, (node, player, answer)) in answers.iter().enumerate() {
        let Some(parent) = game.tree.parent(*node) else {
            continue;
        };
        game.goto(parent);
        let Some(request) = game.pending().cloned() else {
            continue;
        };
        if request.player != *player {
            continue;
        }
        let mode = [InfoMode::Hidden, InfoMode::Decklist, InfoMode::Full][k % 3];
        let knowledge = Knowledge::of(&game, *player);
        let Some((obs, index)) = observe_indexed(&game, &knowledge, *player, mode) else {
            continue;
        };
        let answering = Answering::new(&game, &request, &obs, &index);
        let kind = kind_name(&answering.ask.kind);
        let mut t = tally.lock().unwrap();
        *t.requests.entry(kind.clone()).or_default() += 1;
        drop(t);
        let later: Vec<Answer> = answers[k..].iter().map(|(_, _, a)| a.clone()).collect();
        let x = match &request.kind {
            RequestKind::PayCosts {
                announcing: true,
                x,
                ..
            } => Some(u16::try_from(final_x(*x, &later)).unwrap_or(u16::MAX)),
            RequestKind::UnlessPays { x: Some(x), .. } => Some(*x),
            _ => None,
        };
        let factored =
            factored_name(&game, answer).or_else(|| answering.factored(&request, answer, x));
        let names: Vec<String> = match &factored {
            Some(FactoredAnswer::Name(Some(n))) => vec![n.clone()],
            _ => Vec::new(),
        };
        let at = format!("{name} answer {k} ({kind}, {answer:?})");
        match &factored {
            None => tally.lock().unwrap().unfactored += 1,
            Some(f) => {
                let check = || -> Result<(), String> {
                    let tokens = answering
                        .decompose(&names, f)
                        .map_err(|e| format!("decompose: {e}"))?;
                    let mut m = Machine::new(&answering.ask, &names);
                    for &t in &tokens {
                        assert!(m.choose_token(t));
                    }
                    // An X that is no bucket comes back as the bucket under it (`cantrip_obs::factor::X_BUCKETS`), and one
                    // over the estimate of what the seat could pay (`x_most`, which counts no cost reduction) as the most
                    // under that.
                    let most = match &answering.ask.kind {
                        AskKind::PayCosts { x_most, .. } | AskKind::UnlessPays { x_most, .. } => {
                            *x_most
                        }
                        _ => u16::MAX,
                    };
                    let under = |b: u16, want: u16| {
                        b <= want && (b == want || !X_BUCKETS.contains(&want) || want > most)
                    };
                    let bucketed = match (m.answer(), f) {
                        (Some(FactoredAnswer::X(b)), FactoredAnswer::X(want)) => under(*b, *want),
                        (
                            Some(FactoredAnswer::Unless {
                                pay: true,
                                x: Some(b),
                            }),
                            FactoredAnswer::Unless {
                                pay: true,
                                x: Some(want),
                            },
                        ) => under(*b, *want),
                        _ => false,
                    };
                    if bucketed && m.answer() != Some(f) {
                        tally.lock().unwrap().clamped += 1;
                    }
                    if !bucketed && m.answer().map(canon) != Some(canon(f)) {
                        return Err(format!("the machine made {:?}, not {f:?}", m.answer()));
                    }
                    let made = answering
                        .engine(&game, &request, f)
                        .map_err(|r| format!("refused: {} ({:?})", r.why, r.answer))?;
                    if !payment(f) && normal(&made) != normal(answer) {
                        return Err(format!("made {made:?}"));
                    }
                    Ok(())
                };
                tally.lock().unwrap().trips += 1;
                if let Err(e) = check() {
                    tally.lock().unwrap().failures.push(format!("{at}: {e}"));
                }
            }
        }
        for _ in 0..WALKS {
            let mut m = Machine::new(&answering.ask, &names);
            while let Some(step) = m.step() {
                let legal: Vec<usize> = step.legal().collect();
                assert!(
                    !legal.is_empty(),
                    "{at}: a step with nothing legal: {step:?}"
                );
                let i = legal[random.below(legal.len())];
                m.choose(i);
            }
            let walked = m.answer().unwrap().clone();
            let mut t = tally.lock().unwrap();
            match answering.engine(&game, &request, &walked) {
                Ok(_) => *t.accepted.entry(kind.clone()).or_default() += 1,
                Err(r) if r.global() => *t.global.entry(kind.clone()).or_default() += 1,
                Err(r) => t.local.entry(kind.clone()).or_default().push(format!(
                    "{at}: walked {walked:?}: {} ({:?})",
                    r.why, r.answer
                )),
            }
        }
    }
}

fn corpus(every: usize) {
    let all = scenarios();
    let sample: Vec<PathBuf> = all.into_iter().step_by(every).collect();
    let tally = Mutex::new(Tally::default());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = sample.get(i) else { break };
                    play(path, &tally);
                }
            });
        }
    });
    let t = tally.into_inner().unwrap();
    println!(
        "{} scenarios, {} answers round-tripped; {} answers not in the factoring's terms (an unseen object); {} X answers made at a bucket under them",
        sample.len(),
        t.trips,
        t.unfactored,
        t.clamped
    );
    println!(
        "{:<22} {:>8} {:>9} {:>9} {:>7}",
        "kind", "requests", "accepted", "global", "local"
    );
    for (kind, n) in &t.requests {
        let (a, g, l) = (
            t.accepted.get(kind).copied().unwrap_or(0),
            t.global.get(kind).copied().unwrap_or(0),
            t.local.get(kind).map_or(0, Vec::len),
        );
        let walks = (a + g + l).max(1) as f64;
        println!(
            "{kind:<22} {n:>8} {:>8.1}% {:>8.1}% {:>6.1}%",
            100.0 * a as f64 / walks,
            100.0 * g as f64 / walks,
            100.0 * l as f64 / walks
        );
    }
    let unraised: Vec<&str> = KINDS
        .iter()
        .copied()
        .filter(|k| !t.requests.contains_key(*k))
        .collect();
    println!("kinds no scenario raised: {unraised:?}");
    let local: Vec<&String> = t.local.values().flatten().collect();
    for f in t.failures.iter().take(30) {
        println!("FAIL {f}");
    }
    for l in local.iter().take(30) {
        println!("LOCAL {l}");
    }
    assert!(
        t.failures.is_empty() && local.is_empty(),
        "{} round trips failed, {} walks refused locally",
        t.failures.len(),
        local.len()
    );
}

#[test]
fn a_sample_of_the_corpus_factors() {
    corpus(50);
}

#[test]
#[ignore]
fn the_whole_corpus_factors() {
    corpus(1);
}
