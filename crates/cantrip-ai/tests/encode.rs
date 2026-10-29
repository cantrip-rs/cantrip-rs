//! The encoding (`cantrip_obs::encode`) against the scenario corpus. At every request a scenario answers, in each
//! information mode, every step of the answer given is encoded with the choices before it, and the encoding must fit,
//! read back from its compact form exactly, encode the same twice, and (where no cap was hit) carry the step's options
//! and legal mask and name the objects the options name. Sizes are printed per bucket.
//!
//! Goldens: a few scenarios' last answered request, in each mode, dumped token by token under `tests/golden/encode/`
//! (`CANTRIP_GOLDEN=1` writes them). Cards are mapped to rows by a hash of the name, so the goldens do not move with
//! the card index.
//!
//! The fast test takes a sample of the corpus; `cargo test -p cantrip-ai --test encode -- --ignored` takes all of it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cantrip_ai::{Answering, Knowledge, CardIndex, factored_name, final_x, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::encode::{self, Encoded, Pointee};
use cantrip_obs::factor::{FactoredAnswer, Machine, Step, Token};
use cantrip_obs::{InfoMode, Observation, Ref};
use cantrip_script::GameScript;

const GOLDEN: [&str; 4] = [
    "a-bolted-blocker-carries-damage-into-the-division",
    "abrupt-decay-destroys-an-x-spell-whose-mana-value-is-zero-on-the-battlefield",
    "adeline-and-jacked-rabbit-hand-over-their-attack-triggers-to-order",
    "counterspell-countered-by-counterspell-lets-the-first-spell-resolve",
];

const MODES: [InfoMode; 3] = [InfoMode::Full, InfoMode::Decklist, InfoMode::Hidden];

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

fn hashed(name: &str) -> u32 {
    1 + (encode::fnv1a64(name.as_bytes()) % 4096) as u32
}

/// One answered request: the observation in a mode, and the steps of the answer given (the step and the choices
/// before it), or the first step alone where the answer is not in the factoring's terms.
struct Position {
    obs: Observation,
    steps: Vec<(Step, Vec<Token>)>,
    names: Vec<String>,
}

/// Every answered request of a scenario, in each mode.
fn positions(path: &Path) -> Vec<(usize, InfoMode, Position)> {
    let Ok(script) = GameScript::load(path) else {
        return Vec::new();
    };
    let Ok(result) = cantrip_script::run(&script) else {
        return Vec::new();
    };
    let mut game = result.game;
    let nodes = game.tree.path(game.at);
    let answers: Vec<_> = nodes
        .iter()
        .filter_map(|&n| game.tree.answer(n).map(|(_, p, a)| (n, *p, a.clone())))
        .collect();
    let mut out = Vec::new();
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
        let knowledge = Knowledge::of(&game, *player);
        for mode in MODES {
            let Some((obs, index)) = observe_indexed(&game, &knowledge, *player, mode) else {
                continue;
            };
            let answering = Answering::new(&game, &request, &obs, &index);
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
            let mut m = Machine::new(&answering.ask, &names);
            let mut steps = Vec::new();
            let tokens = factored
                .as_ref()
                .and_then(|f| answering.decompose(&names, f).ok())
                .unwrap_or_default();
            for t in tokens {
                let Some(step) = m.step() else { break };
                steps.push((step.clone(), m.chosen().to_vec()));
                if !m.choose_token(t) {
                    break;
                }
            }
            if steps.is_empty()
                && let Some(step) = m.step()
            {
                steps.push((step.clone(), Vec::new()));
            }
            let names = m.names();
            out.push((k, mode, Position { obs, steps, names }));
        }
    }
    out
}

#[derive(Default)]
struct Sizes {
    positions: usize,
    by_bucket: BTreeMap<usize, (usize, usize, usize, usize)>,
    cap_hits: usize,
    longest: usize,
    failures: Vec<String>,
}

fn check(
    at: &str,
    obs: &Observation,
    step: &Step,
    chosen: &[Token],
    names: &[String],
) -> Result<Encoded, String> {
    let e = encode::encode(obs, step, chosen, names, &hashed);
    let again = encode::encode(obs, step, chosen, names, &hashed);
    if !encode::same_tensors(&e, &again) || e.origin != again.origin {
        return Err(format!("{at}: encodes differently twice"));
    }
    if e.len > encode::MAX_TOKENS
        || e.bucket != *encode::BUCKETS.iter().find(|&&b| b >= e.len).unwrap()
    {
        return Err(format!("{at}: {} tokens in bucket {}", e.len, e.bucket));
    }
    let back = encode::from_compact(&encode::to_compact(&e))
        .ok_or(format!("{at}: the compact form does not read"))?;
    if !encode::same_tensors(&e, &back) {
        return Err(format!("{at}: the compact form reads back otherwise"));
    }
    let d = encode::decode(&e);
    if d.zones.values().sum::<u32>() as usize != obs.objects.len() {
        return Err(format!(
            "{at}: {:?} objects by zone, not {}",
            d.zones,
            obs.objects.len()
        ));
    }
    if !e.cap_hit {
        if e.options != step.tokens.len()
            || (0..e.options).any(|k| e.legal[e.first_option + k] != step.mask[k])
        {
            return Err(format!(
                "{at}: the options or their mask are not the step's"
            ));
        }
        for (k, (t, (tok, a, b, n))) in step.tokens.iter().zip(&d.options).enumerate() {
            let same = |r: Ref, p: &Pointee| match (r, p) {
                (Ref::None, Pointee::None) => true,
                (Ref::Player(x), Pointee::Player(y)) => x == *y,
                (Ref::Object(x), Pointee::Object(y)) => x == *y,
                _ => false,
            };
            // The objects an option names are pinned: they are only pooled when nothing else fits.
            if *tok != t.kind || !same(t.a, a) || !same(t.b, b) || (t.n < 2048 && *n != t.n) {
                return Err(format!(
                    "{at}: option {k} is {t:?}, decoded {:?}",
                    (tok, a, b, n)
                ));
            }
        }
    }
    Ok(e)
}

fn corpus(every: usize) {
    let sample: Vec<PathBuf> = scenarios().into_iter().step_by(every).collect();
    let sizes = Mutex::new(Sizes::default());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = sample.get(i) else { break };
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    for (k, mode, p) in positions(path) {
                        for (j, (step, chosen)) in p.steps.iter().enumerate() {
                            let at = format!("{name} answer {k} {mode:?} step {j}");
                            let result = check(&at, &p.obs, step, chosen, &p.names);
                            let mut s = sizes.lock().unwrap();
                            match result {
                                Ok(e) => {
                                    let bytes = encode::to_compact(&e).len();
                                    s.positions += 1;
                                    s.cap_hits += e.cap_hit as usize;
                                    s.longest = s.longest.max(e.len);
                                    let b = s.by_bucket.entry(e.bucket).or_insert((
                                        0,
                                        0,
                                        usize::MAX,
                                        0,
                                    ));
                                    b.0 += 1;
                                    b.1 += bytes;
                                    b.2 = b.2.min(bytes);
                                    b.3 = b.3.max(bytes);
                                }
                                Err(f) => s.failures.push(f),
                            }
                        }
                    }
                }
            });
        }
    });
    let s = sizes.into_inner().unwrap();
    println!(
        "{} scenarios, {} positions encoded, {} hit the cap, the longest {} tokens",
        sample.len(),
        s.positions,
        s.cap_hits,
        s.longest
    );
    println!(
        "{:>6} {:>9} {:>12} {:>12} {:>12} {:>12}",
        "bucket", "positions", "dense bytes", "compact min", "compact mean", "compact max"
    );
    for (bucket, (n, total, min, max)) in &s.by_bucket {
        let dense = bucket * (1 + 4 + encode::CATS + encode::PTRS + 2 * encode::NUMS + 2);
        println!(
            "{bucket:>6} {n:>9} {dense:>12} {min:>12} {:>12} {max:>12}",
            total / n.max(&1)
        );
    }
    for f in s.failures.iter().take(30) {
        println!("FAIL {f}");
    }
    assert!(
        s.failures.is_empty(),
        "{} encodings failed",
        s.failures.len()
    );
}

#[test]
fn a_sample_of_the_corpus_encodes() {
    corpus(50);
}

#[test]
#[ignore]
fn the_whole_corpus_encodes() {
    corpus(1);
}

#[test]
fn goldens() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/encode");
    let write = std::env::var_os("CANTRIP_GOLDEN").is_some();
    let mut wrong = Vec::new();
    for name in GOLDEN {
        let all = positions(&root().join(format!("scenarios/{name}.cantrip")));
        let last = all
            .iter()
            .map(|(k, _, _)| *k)
            .max()
            .unwrap_or_else(|| panic!("{name} answers no request"));
        let mut text = format!("# {name}, answer {last}\n");
        for (_, mode, p) in all.iter().filter(|(k, _, _)| *k == last) {
            let (step, chosen) = p.steps.first().cloned().unwrap_or_default();
            let e = encode::encode(&p.obs, &step, &chosen, &p.names, &hashed);
            text += &format!("\n## {mode:?}\n{}", encode::dump(&e));
        }
        let path = dir.join(format!("{name}.txt"));
        if write {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &text).unwrap();
        } else if std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n")
            != text
        {
            wrong.push(name);
        }
    }
    assert!(
        wrong.is_empty(),
        "the encodings of {wrong:?} are not their goldens: rerun with CANTRIP_GOLDEN=1 if the change is meant"
    );
}

/// The scenario corpus's positions encoded with the engine's card index: every visible card the observations name has
/// a row. A name no card has is one an effect gave the object (The Irencrag named Everflame, CR 201.3), and has no row to find.
#[test]
fn the_cards_of_the_sample_have_rows() {
    let table = CardIndex::engine();
    let card_names: std::collections::HashSet<&str> = cantrip_engine::cards().cards.iter().map(|c| c.name).collect();
    let mut named = 0;
    let mut missing = BTreeMap::<String, usize>::new();
    for path in scenarios().into_iter().step_by(200) {
        for (_, mode, p) in positions(&path) {
            if mode != InfoMode::Full {
                continue;
            }
            for o in &p.obs.objects {
                if let cantrip_obs::ObjectObs::Visible(v) = o
                    && let Some(n) = &v.name
                {
                    named += 1;
                    if card_names.contains(n.as_str()) && table.row(n) == encode::UNKNOWN_CARD {
                        *missing.entry(n.clone()).or_default() += 1;
                    }
                }
            }
        }
    }
    println!(
        "{named} named objects, {} without a row: {missing:?}",
        missing.values().sum::<usize>()
    );
    assert!(named > 0);
    assert!(missing.is_empty(), "named objects with no row: {missing:?}");
}
