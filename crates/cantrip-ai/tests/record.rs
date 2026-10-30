//! The recorder (`cantrip_ai::record`) and its shards (`cantrip_ai::shard`) against the scenario corpus. A sample of
//! the corpus's games is recorded, in each information mode, written into shards and read back: the games and samples
//! read are the ones written, every chosen option is a legal one of its position, the header says what made them, and
//! recording the same game twice makes the same bytes. The answers that made no sample are counted by request kind
//! and why, printed, and must be none but the kinds listed in `EXPECTED`.
//!
//! The fast test takes a sample of the corpus; `cargo test -p cantrip-ai --test record -- --ignored` takes all of it.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cantrip_ai::record::{Dealt, Ending, Failures, record, table};
use cantrip_ai::shard::{self, GameRecord, Header, ShardWriter, game_bytes, mode_name};
use cantrip_obs::InfoMode;
use cantrip_obs::encode::{SCHEMA_VERSION, encoder_hash, from_compact};
use cantrip_script::GameScript;

const MODES: [InfoMode; 3] = [InfoMode::Full, InfoMode::Decklist, InfoMode::Hidden];

/// Answers the recorder skips, by "kind/why". A creature type chosen from all of them (some 300) has more options than
/// a position's 128 tokens hold, and the encoder cuts options past the cap (`cantrip_obs::encode`); when the type
/// chosen is one cut, the answer makes no sample. 58 answers of the whole corpus's 7404 scenarios, at present
/// (`docs/known-issues.md`, KI-78).
const EXPECTED: [&str; 1] = ["ChooseSubtype/chosen option cut by the cap"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scenarios(every: usize) -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(root().join("scenarios"))
        .expect("scenarios")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "cantrip"))
        .collect();
    all.sort();
    all.into_iter().step_by(every).collect()
}

/// A scenario's game recorded in a mode, as the host would record it once it ended or was closed.
fn recorded(path: &Path, mode: InfoMode) -> Option<(GameRecord, Failures)> {
    let script = GameScript::load(path).ok()?;
    let game = cantrip_script::run(&script).ok()?.game;
    let names = (0..game.state.players.len())
        .map(|i| format!("Seat{}", i + 1))
        .collect();
    let dealt = Dealt {
        seed: 0,
        names,
        decks: Vec::new(),
        auto_start: false,
        ..Dealt::default()
    };
    let ending = Ending::of(&game, !game.is_over());
    Some(record(game, &dealt, &ending, mode, table()))
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cantrip-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Records the scenarios `sample`, in a directory of its own (`name`), as the tests run at once.
fn corpus(name: &str, sample: Vec<PathBuf>) {
    let failures = Mutex::new(Failures::new());
    let problems = Mutex::new(Vec::<String>::new());
    let totals = Mutex::new((0usize, 0usize, 0usize));
    let next: [std::sync::atomic::AtomicUsize; 3] = Default::default();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let dir = temp(name);
    std::thread::scope(|s| {
        for t in 0..threads {
            let (sample, failures, problems, totals, next, dir) =
                (&sample, &failures, &problems, &totals, &next, &dir);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(s, move || {
                    for (m, mode) in MODES.into_iter().enumerate() {
                        let header = Header::new(table(), "test", "scenarios", mode);
                        let mut writer = ShardWriter::new(&dir.join(mode_name(mode)), &format!("t{t}"), header.clone(), 1 << 20, 7).unwrap();
                        let mut games = Vec::new();
                        loop {
                            let i = next[m].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(path) = sample.get(i) else { break };
                            let name = path.file_stem().unwrap().to_string_lossy().to_string();
                            let Some((game, f)) = recorded(path, mode) else { continue };
                            if mode == InfoMode::Full {
                                let mut all = failures.lock().unwrap();
                                for (k, n) in f {
                                    *all.entry(k).or_default() += n;
                                }
                            }
                            for (j, s) in game.samples.iter().enumerate() {
                                let ok = from_compact(&s.position).is_some_and(|e| s.chosen < e.options as u16 && e.legal[e.first_option + s.chosen as usize]);
                                if !ok {
                                    problems.lock().unwrap().push(format!("{name} {mode:?} sample {j}: the chosen option {} is not a legal one", s.chosen));
                                }
                            }
                            writer.write(&game).unwrap();
                            games.push(game);
                        }
                        writer.close().unwrap();
                        let mut read = Vec::new();
                        for path in &writer.written {
                            let (h, g) = shard::read(path).unwrap();
                            if h != header {
                                problems.lock().unwrap().push(format!("{}: the header read is {h:?}", path.display()));
                            }
                            read.extend(g);
                        }
                        if read.len() != games.len() || read.iter().zip(&games).any(|(a, b)| game_bytes(a) != game_bytes(b)) {
                            problems.lock().unwrap().push(format!("{mode:?} thread {t}: {} games written, {} read, or not the same", games.len(), read.len()));
                        }
                        let mut tl = totals.lock().unwrap();
                        tl.0 += games.len();
                        tl.1 += games.iter().map(|g| g.samples.len()).sum::<usize>();
                        tl.2 += writer.written.len();
                    }
                })
                .unwrap();
        }
    });
    let _ = std::fs::remove_dir_all(&dir);
    let (games, samples, shards) = totals.into_inner().unwrap();
    let failures = failures.into_inner().unwrap();
    println!(
        "{} scenarios: {games} games recorded in three modes, {samples} samples, {shards} shards",
        sample.len()
    );
    for (k, n) in &failures {
        println!("not recorded (Full): {n} x {k}");
    }
    let problems = problems.into_inner().unwrap();
    for p in problems.iter().take(30) {
        println!("FAIL {p}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
    assert!(samples > 0);
    let unexpected: Vec<_> = failures
        .keys()
        .filter(|k| !EXPECTED.contains(&k.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "answers not recorded: {unexpected:?}"
    );
}

#[test]
fn a_sample_of_the_corpus_records_and_reads_back() {
    corpus("sample", scenarios(50));
}

/// Scenarios that once failed, checked whatever the sample takes.
#[test]
fn scenarios_that_once_failed_record() {
    corpus(
        "once-failed",
        [
            // Bot1 controls Bot2 during their turn (CR 723.1) and sees the card they draw (723.4): what a seat saw is what
            // it could look into then, not now that the control has ended (`Event::PlayerControlled`).
            "emrakul-the-promised-end-costs-one-less-for-each-card-type-in-your-graveyard",
            "emrakul-the-promised-end-controls-the-next-turn-then-gives-an-extra-one-and-is-protected-from-instants",
            "emrakul-the-promised-end-countered-still-controls-the-next-turn-and-gives-an-extra-one",
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
fn the_whole_corpus_records_and_reads_back() {
    corpus("corpus", scenarios(1));
}

/// Recording is a walk over a copy: the same game recorded twice is the same bytes, and the game's own line ends where
/// it ended.
#[test]
fn recording_is_deterministic_and_leaves_the_game() {
    let name = "a-bolted-blocker-carries-damage-into-the-division";
    let path = root().join(format!("scenarios/{name}.cantrip"));
    let script = GameScript::load(&path).unwrap();
    let game = cantrip_script::run(&script).unwrap().game;
    let before = (game.at, format!("{:?}", game.state));
    let dealt = Dealt {
        seed: 0,
        names: vec!["A".into(), "B".into()],
        decks: vec!["x".into(), "y".into()],
        auto_start: false,
        ..Dealt::default()
    };
    let ending = Ending::of(&game, !game.is_over());
    let (a, _) = record(game.clone(), &dealt, &ending, InfoMode::Full, table());
    let (b, _) = record(game.clone(), &dealt, &ending, InfoMode::Full, table());
    assert!(!a.samples.is_empty());
    assert_eq!(game_bytes(&a), game_bytes(&b));
    assert_eq!(before, (game.at, format!("{:?}", game.state)));
    assert_eq!(a.meta.decks, vec!["x".to_string(), "y".to_string()]);
    assert_eq!(a.meta.modes, vec!["full".to_string(), "full".to_string()]);
}

#[test]
fn the_header_says_what_made_the_shard() {
    let dir = temp("header");
    let header = Header::new(table(), "tactical:abc", "1..10", InfoMode::Decklist);
    assert_eq!(header.schema, SCHEMA_VERSION);
    assert_eq!(header.encoder_hash, format!("{:016x}", encoder_hash()));
    assert_eq!(header.mode, "decklist");
    assert!(!header.commit.is_empty());
    let mut writer = ShardWriter::new(&dir, "h", header.clone(), 0, 0).unwrap();
    let game = GameRecord {
        meta: Default::default(),
        samples: Vec::new(),
    };
    writer.write(&game).unwrap();
    // Not readable as a shard until closed.
    assert!(shard::shards(&dir).unwrap().is_empty());
    writer.close().unwrap();
    let files = shard::shards(&dir).unwrap();
    assert_eq!(files.len(), 1);
    let (h, games) = shard::read(&files[0]).unwrap();
    assert_eq!(h, header);
    assert_eq!(games.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
