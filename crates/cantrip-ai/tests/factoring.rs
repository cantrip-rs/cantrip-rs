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
        Answer::Shares(x) => {
            x.retain(|s| s.2 != 0);
            x.sort();
        }
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
            } => Some(final_x(*x, &later)),
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
                        _ => u32::MAX,
                    };
                    let under = |b: u32, want: u32| {
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
    check(scenarios().into_iter().step_by(every).collect());
}

fn check(sample: Vec<PathBuf>) {
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

/// Scenarios that once failed, checked whatever the sample takes.
#[test]
fn scenarios_that_once_failed_factor() {
    check(
        [
            // "Remove up to three counters": two, of two kinds from one creature, is an answer the factoring makes.
            "glissa-sunslayer-removes-fewer-counters-from-a-creature",
            // A creature that must attack may stay home where attacking costs something (CR 508.1d) or what it must
            // attack is gone or taxed.
            "a-creature-that-attacks-each-combat-if-able-is-not-made-to-pay-ghostly-prison",
            "gideon-battle-forged-gone-frees-the-creature-told-to-attack-him",
            "ursine-monstrosity-told-to-attack-a-taxed-player-may-attack-nothing",
            // Targets with different mana values together (CR 601.2c): the validator's, a refusal walks may meet.
            "agadeems-awakening-at-x-zero-takes-one-value-zero-card-and-an-x-in-a-graveyard-card-counts-as-zero",
            // Creatures tapped for a total power (CR 107.1b: a negative one takes from it), chosen once the mana is paid
            // (CR 601.2g-h), and a cost made unpayable while paying taken back (CR 733.1).
            "crew-counts-a-negative-power-against-the-total",
            "smugglers-copter-crew-taxed-by-anointed-peacekeeper-cannot-tap-the-elf-that-paid-the-mana",
            "we-say-thee-nay-teamwork-made-unpayable-by-tapping-an-elf-for-mana-is-taken-back",
            "we-say-thee-nay-teamwork-taps-a-summoning-sick-creature-and-another-for-total-power-two",
            // Creatures that can't attack alone (CR 508.1c): a walk declaring one alone is the validator's refusal.
            "pet-sehlat-two-attack-together",
        ]
        .iter()
        .map(|n| {
            let path = root().join("scenarios").join(format!("{n}.cantrip"));
            assert!(path.exists(), "{n} is no scenario");
            path
        })
        .collect(),
    );
}

#[test]
#[ignore]
fn the_whole_corpus_factors() {
    corpus(1);
}

#[test]
fn combat_zero_shares_factor_without_hiding_invalid_assignments() {
    let script = GameScript::parse(
        include_str!("data/zero-share-foundation.cantrip"),
        "zero-share-foundation",
    ).unwrap();
    let mut game = cantrip_script::run(&script).unwrap().game;
    let answers: Vec<_> = game
        .tree
        .path(game.at)
        .iter()
        .filter_map(|&n| game.tree.answer(n).map(|(_, p, a)| (n, *p, a.clone())))
        .collect();
    let (node, player, answer) = answers.into_iter().find(|(_, _, a)| matches!(a, Answer::Shares(v) if v.iter().any(|x| x.2 == 0) && v.iter().any(|x| x.2 > 0))).expect("scenario must assign positive and zero shares");
    game.goto(game.tree.parent(node).unwrap());
    let request = game.pending().cloned().unwrap();
    assert!(check_answer(&game, &request, &answer).is_ok());
    let knowledge = Knowledge::of(&game, player);
    let (obs, index) = observe_indexed(&game, &knowledge, player, InfoMode::Full).unwrap();
    let answering = Answering::new(&game, &request, &obs, &index);
    let factored = answering.factored(&request, &answer, None).unwrap();
    let FactoredAnswer::Shares(shares) = &factored else {
        panic!("shares expected")
    };
    assert!(
        shares.iter().all(|s| s.2 > 0),
        "zero damage is not a separate assignment: {shares:?}"
    );
    let tokens = answering.decompose(&[], &factored).unwrap();
    let mut machine = Machine::new(&answering.ask, &[]);
    for token in tokens {
        assert!(machine.choose_token(token));
    }
    assert_eq!(machine.answer().map(canon), Some(canon(&factored)));
    let replay = answering.engine(&game, &request, &factored).unwrap();
    assert_eq!(normal(&replay), normal(&answer));

    let Answer::Shares(original) = &answer else {
        unreachable!()
    };
    let mut invalid = original.clone();
    invalid.push((ObjectId(u32::MAX), None, 0));
    let invalid = Answer::Shares(invalid);
    assert!(
        check_answer(&game, &request, &invalid).is_err(),
        "unknown zero source remains illegal"
    );
    assert!(
        answering.factored(&request, &invalid, None).is_none(),
        "invalid zero source must not disappear"
    );
    let mut invalid = original.clone();
    invalid.push((original[0].0, Some(original[0].0), 0));
    let invalid = Answer::Shares(invalid);
    assert!(
        check_answer(&game, &request, &invalid).is_err(),
        "zero damage to a non-blocker remains illegal"
    );
    assert!(
        answering.factored(&request, &invalid, None).is_none(),
        "invalid zero recipient must not disappear"
    );
    let mut invalid = original.clone();
    invalid.push((original[0].0, None, 0));
    let invalid = Answer::Shares(invalid);
    assert!(
        check_answer(&game, &request, &invalid).is_err(),
        "a zero share to the defender requires permission"
    );
    assert!(answering.factored(&request, &invalid, None).is_none());

    // A request may contain more than one division for a source. Every matching division must
    // permit the recipient, just as the engine validator requires.
    let mut legal_duplicate_request = request.clone();
    let RequestKind::AssignCombatDamage {
        divisions: legal_divisions,
    } = &mut legal_duplicate_request.kind
    else {
        panic!("combat request expected")
    };
    let same_division = legal_divisions
        .iter()
        .find(|d| d.source == original[0].0)
        .unwrap()
        .clone();
    legal_divisions.push(same_division);
    assert!(check_answer(&game, &legal_duplicate_request, &answer).is_ok());
    assert!(
        answering
            .factored(&legal_duplicate_request, &answer, None)
            .is_some()
    );

    let mut duplicate_request = request.clone();
    let RequestKind::AssignCombatDamage { divisions } = &mut duplicate_request.kind else {
        panic!("combat request expected")
    };
    let (source, zero_target, _) = *original.iter().find(|s| s.2 == 0).unwrap();
    let mut duplicate = divisions
        .iter()
        .find(|d| d.source == source)
        .unwrap()
        .clone();
    let zero_target = zero_target.expect("zero target is a blocker");
    let position = duplicate
        .creatures
        .iter()
        .position(|&c| c == zero_target)
        .unwrap();
    duplicate.creatures.remove(position);
    duplicate.lethal.remove(position);
    divisions.push(duplicate);
    assert!(check_answer(&game, &duplicate_request, &answer).is_err());
    assert!(
        check_answer(&game, &duplicate_request, &normal(&answer)).is_ok(),
        "the positive recipient is legal in both divisions; dropping zero alone would hide the invalid answer"
    );
    assert!(
        answering
            .factored(&duplicate_request, &answer, None)
            .is_none(),
        "a zero share rejected by any matching division must not disappear"
    );

    let mut invalid_positive = original.clone();
    invalid_positive[0].2 += 1;
    let invalid_positive = Answer::Shares(invalid_positive);
    assert!(check_answer(&game, &request, &invalid_positive).is_err());
    let invalid_factored = answering
        .factored(&request, &invalid_positive, None)
        .unwrap();
    assert!(
        answering
            .engine(&game, &request, &invalid_factored)
            .is_err(),
        "positive over-assignment is still refused"
    );

    // Replay both legal forms from the same actual game position. Damage and later rule events must agree.
    let parent = game.tree.parent(node).unwrap();
    let start = game.log.len();
    game.answer(request.id, player, answer.clone()).unwrap();
    game.run();
    let events = game.log[start..].to_vec();
    game.goto(parent);
    let start = game.log.len();
    let again = game.pending().cloned().unwrap();
    game.answer(again.id, player, replay).unwrap();
    game.run();
    assert_eq!(
        &game.log[start..],
        events.as_slice(),
        "zero-share normalization must not alter the emitted events"
    );
}
