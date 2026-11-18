//! Self-play under its governor (`cantrip_ai_host::governor`), with the tiny model at 8 simulations and short games:
//!
//! - a game's record is the same bytes whatever the lanes, the scaling and the memory admission that ran it;
//! - a stop for memory closes every lane's shard with the games it finished, drops the games it stopped, and a resumed
//!   run plays exactly the rest; an interrupt lets the games in flight finish;
//! - `.part`s cut inside a game keep the games before the cut on a resume, each in a shard of its own, and a recovery
//!   interrupted between its steps and done again leaves each game once;
//! - a resume writes a game a hard kill left out of an `--also-record` directory there, and nowhere twice;
//! - a live run's directory is refused, and a dead run's lock taken over;
//! - the binary exits 3 when the governor stops it, and when its stop file asks it to;
//! - games split between runs by grant files are the reference's games, each once, and a run waits for its grant to
//!   grow until it says `end`;
//! - `recover` closes a killed run's `.part`s and plays nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cantrip_ai::shard::{self, game_bytes};
use cantrip_ai::{Evaluator, Prior, TractEvaluator, record::table};
use cantrip_ai_host::governor::{Govern, Memory, Probe, Scale, gb};
use cantrip_ai_host::selfplay::{self, Options, Pairing, Run};
use cantrip_obs::InfoMode;
use serde_json::Value;

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pairing() -> Pairing {
    let deck = |n: &str| selfplay::load_deck(&root().join("decks").join(format!("{n}.txt"))).expect("the decklist");
    Pairing { names: ["rw-aggro".into(), "ub-reanimator".into()], lists: [deck("rw-aggro"), deck("ub-reanimator")] }
}

fn options() -> Options {
    Options { model: "tiny".into(), simulations: 8, sampled: 4, max_turns: 4, ..Options::default() }
}

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(cantrip_ai_host::STACK_SIZE).spawn(f).unwrap().join().unwrap()
}

fn out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cantrip-governor-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

const GAMES: usize = 6;

fn run_in(out: &Path, lanes: usize, govern: Govern, resume: bool) -> Value {
    try_run(out, lanes, govern, resume, 3, &options()).unwrap()
}

fn try_run(out: &Path, lanes: usize, govern: Govern, resume: bool, shard_games: usize, options: &Options) -> Result<Value, String> {
    try_run_granted(out, lanes, govern, resume, shard_games, options, None)
}

fn try_run_granted(out: &Path, lanes: usize, govern: Govern, resume: bool, shard_games: usize, options: &Options, grant: Option<PathBuf>) -> Result<Value, String> {
    let prior = Prior::from_decks(&root().join("decks")).unwrap();
    let run = Run {
        out: out.to_path_buf(),
        games: GAMES,
        limit: None,
        lanes,
        seed: 7,
        pairings: vec![pairing()],
        producer: "selfplay:tiny@8".into(),
        shard_mb: 64,
        shard_games,
        resume,
        govern,
        grant,
        host: Some("test".into()),
    };
    selfplay::run(&run, options, &prior, || {
        Ok(Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?) as Box<dyn Evaluator>)
    })
}

/// `try_run` on a big stack, unwrapped.
fn run_big(out: &Path, lanes: usize, resume: bool, shard_games: usize, options: Options) -> Value {
    let out = out.to_path_buf();
    on_big_stack(move || try_run(&out, lanes, Govern::default(), resume, shard_games, &options).unwrap())
}

/// `path` cut `bytes` short and left as the `.part` an interrupted writer leaves.
fn cut(path: &Path, bytes: usize) -> PathBuf {
    let whole = std::fs::read(path).unwrap();
    let part = path.with_file_name(format!("{}.part", path.file_name().unwrap().to_string_lossy()));
    std::fs::write(&part, &whole[..whole.len() - bytes]).unwrap();
    std::fs::remove_file(path).unwrap();
    part
}

fn recovered(dir: &Path) -> usize {
    shard::shards(dir).unwrap().iter().filter(|p| p.file_name().unwrap().to_string_lossy().contains(".recovered.")).count()
}

/// Every game's bytes in `dir`'s shards, by seed; a seed twice is a failure.
fn games(dir: &Path) -> BTreeMap<u64, Vec<u8>> {
    let mut all = BTreeMap::new();
    for path in shard::shards(dir).unwrap() {
        for g in shard::read(&path).unwrap().1 {
            assert!(all.insert(g.meta.seed, game_bytes(&g)).is_none(), "seed {} twice", g.meta.seed);
        }
    }
    all
}

fn parts(dir: &Path) -> usize {
    std::fs::read_dir(dir).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".part")).count()
}

/// The reference: the games played one at a time, ungoverned.
fn reference() -> BTreeMap<u64, Vec<u8>> {
    static REFERENCE: Mutex<Option<BTreeMap<u64, Vec<u8>>>> = Mutex::new(None);
    let mut r = REFERENCE.lock().unwrap_or_else(|e| e.into_inner());
    r.get_or_insert_with(|| {
        let dir = out("reference");
        on_big_stack({
            let dir = dir.clone();
            move || run_in(&dir, 1, Govern::default(), false)
        });
        let g = games(&dir);
        assert_eq!(g.len(), GAMES);
        let _ = std::fs::remove_dir_all(&dir);
        g
    })
    .clone()
}

/// Memory that has room for one more game at every other lane's asking (the governor's own readings always roomy):
/// admissions deferred whenever a game is running, then let in.
struct Flicker(AtomicUsize);

impl Probe for Flicker {
    fn memory(&self) -> Option<Memory> {
        let lane = std::thread::current().name().is_some_and(|n| n != "selfplay-governor");
        let n = if lane { self.0.fetch_add(1, Ordering::SeqCst) } else { 0 };
        let available = if n % 2 == 0 { gb(100.0) } else { gb(6.5) };
        Some(Memory { private: gb(1.0), available, commit_headroom: gb(100.0) })
    }

    fn cpu(&self) -> Option<(u64, u64)> {
        let n = self.0.load(Ordering::SeqCst) as u64;
        Some((n, 2 * n + 1))
    }
}

#[test]
fn a_games_record_does_not_depend_on_the_lanes_the_scaling_or_memory() {
    let want = reference();
    let dir = out("flicker");
    let govern = Govern {
        reserve: gb(6.0),
        commit_reserve: gb(6.0),
        per_game: gb(1.0),
        tick: Duration::from_millis(3),
        scale: Some(Scale { start: 1, step: 1, window: Duration::from_millis(40), ..Scale::default() }),
        probe: Arc::new(Flicker(AtomicUsize::new(0))),
        ..Govern::default()
    };
    let report = on_big_stack({
        let dir = dir.clone();
        move || run_in(&dir, 3, govern, false)
    });
    assert_eq!(report["Games"], GAMES as u64, "{report}");
    assert!(report["Stopped"].is_null(), "{report}");
    let governor = &report["Governor"];
    assert!(governor["Deferred"]["Admissions"].as_u64().unwrap() > 0, "memory deferred some admissions: {governor}");
    assert!(governor["LanesRunning"]["Max"].as_u64().unwrap() >= 1, "{governor}");
    assert_eq!(games(&dir), want, "the same games, byte for byte, by seed");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Memory under the floor (or an interrupt, with `interrupt`) once a lane has written its first game.
struct StopAfterFirst {
    dir: PathBuf,
    interrupt: Option<Arc<AtomicU32>>,
}

impl Probe for StopAfterFirst {
    fn memory(&self) -> Option<Memory> {
        let written = std::fs::read_dir(&self.dir).is_ok_and(|d| {
            d.filter_map(Result::ok).any(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.ends_with(".part") || n.ends_with(".shard.zst")
            })
        });
        let low = written && self.interrupt.is_none();
        if written && let Some(i) = &self.interrupt {
            i.store(1, Ordering::SeqCst);
        }
        Some(Memory {
            private: gb(1.0),
            available: if low { gb(1.0) } else { gb(100.0) },
            commit_headroom: gb(100.0),
        })
    }
}

fn stopping(dir: &Path, interrupt: bool) -> Govern {
    let interrupts = Arc::new(AtomicU32::new(0));
    Govern {
        floor: gb(2.0),
        tick: Duration::from_millis(3),
        probe: Arc::new(StopAfterFirst { dir: dir.to_path_buf(), interrupt: interrupt.then(|| interrupts.clone()) }),
        interrupts,
        ..Govern::default()
    }
}

#[test]
fn a_stop_for_memory_closes_the_shards_and_a_resume_plays_the_rest() {
    let want = reference();
    let dir = out("floor");
    let report = on_big_stack({
        let (dir, govern) = (dir.clone(), stopping(&dir, false));
        move || run_in(&dir, 2, govern, false)
    });
    let stopped = &report["Stopped"];
    assert!(stopped["Reason"].as_str().unwrap().contains("floor"), "{report}");
    assert_eq!(stopped["ExitCode"], 3);
    assert_eq!(parts(&dir), 0, "every lane's open shard is closed");
    let first = games(&dir);
    assert_eq!(first.len() as u64, report["Games"].as_u64().unwrap(), "{report}");
    assert!(!first.is_empty() && first.len() < GAMES, "{} games before the stop: {report}", first.len());
    for (seed, bytes) in &first {
        assert_eq!(Some(bytes), want.get(seed), "a game finished before the stop is the game");
    }
    // A shard holding fewer than `shard_games`: closed partly full, and counted for the games it holds.
    let resumed = on_big_stack({
        let dir = dir.clone();
        move || run_in(&dir, 2, Govern::default(), true)
    });
    assert_eq!(resumed["Resumed"], first.len() as u64, "{resumed}");
    assert_eq!(resumed["GamesThisRun"], (GAMES - first.len()) as u64, "{resumed}");
    assert!(resumed["Stopped"].is_null());
    assert_eq!(games(&dir), want, "every game once, each as it would have been");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_interrupt_lets_the_games_in_flight_finish() {
    let want = reference();
    let dir = out("interrupt");
    let report = on_big_stack({
        let (dir, govern) = (dir.clone(), stopping(&dir, true));
        move || run_in(&dir, 2, govern, false)
    });
    assert_eq!(report["Stopped"]["Reason"], "interrupted", "{report}");
    assert_eq!(report["Stopped"]["GamesStopped"], 0, "{report}");
    assert_eq!(parts(&dir), 0);
    let first = games(&dir);
    assert_eq!(first.len() as u64, report["Games"].as_u64().unwrap());
    let _ = on_big_stack({
        let dir = dir.clone();
        move || run_in(&dir, 3, Govern::default(), true)
    });
    assert_eq!(games(&dir), want);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parts_cut_inside_a_game_keep_the_games_before_the_cut_each_in_its_own_shard() {
    let want = reference();
    let dir = out("cut");
    run_big(&dir, 1, false, 2, options());
    // Six games in three shards of two: two of them cut inside their last game and left as `.part`s, recovered in one
    // resume (each into a shard named after it, so neither overwrites the other).
    let shards = shard::shards(&dir).unwrap();
    assert_eq!(shards.len(), 3);
    let first = cut(&shards[0], 7);
    let saved = std::fs::read(&first).unwrap();
    cut(&shards[2], 7);
    let report = run_big(&dir, 2, true, 2, options());
    assert_eq!(report["Resumed"], 4, "the whole game before each cut kept: {report}");
    assert_eq!(report["GamesThisRun"], 2, "{report}");
    assert_eq!(parts(&dir), 0, "the cut .parts are gone");
    assert_eq!(recovered(&dir), 2, "each in a recovered shard of its own");
    assert_eq!(games(&dir), want);

    // A recovery interrupted after its shard was renamed into place but before the `.part` was removed, with another's
    // temporary file left behind: done again, it leaves every game once.
    std::fs::write(&first, &saved).unwrap();
    let leftover = dir.join("selfplay-l0-1-2-00009.recovered.tmp");
    std::fs::write(&leftover, b"half a shard").unwrap();
    let report = run_big(&dir, 2, true, 2, options());
    assert_eq!(report["Resumed"], GAMES as u64, "{report}");
    assert_eq!(report["GamesThisRun"], 0, "{report}");
    assert_eq!(parts(&dir), 0);
    assert!(!leftover.exists(), "the leftover temporary file is removed");
    assert_eq!(recovered(&dir), 2);
    assert_eq!(games(&dir), want, "no game twice");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_resume_fills_in_what_a_hard_kill_left_out_of_also_record() {
    let dir = out("also");
    let also = Options { also: vec![InfoMode::Decklist], ..options() };
    run_big(&dir, 1, false, 3, also.clone());
    let decklist = dir.join("decklist");
    let (want, want_also) = (games(&dir), games(&decklist));
    assert_eq!((want.len(), want_also.len()), (GAMES, GAMES));
    // Killed between the writes: the second also-shard lost (its games only in the run's own shards), and the first of
    // the run's own lost (its games only in the also-directory).
    std::fs::remove_file(&shard::shards(&decklist).unwrap()[1]).unwrap();
    std::fs::remove_file(&shard::shards(&dir).unwrap()[0]).unwrap();
    let report = run_big(&dir, 2, true, 3, also);
    assert_eq!(report["Resumed"], 3, "{report}");
    assert_eq!(report["GamesThisRun"], 3, "the three the run's own shards lacked: {report}");
    assert_eq!(games(&dir), want, "each game once in the run's own shards");
    assert_eq!(games(&decklist), want_also, "and once in the also-directory, played again where it lacked them");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_live_runs_directory_is_refused_and_a_dead_ones_lock_is_taken_over() {
    let dir = out("lock");
    std::fs::create_dir_all(&dir).unwrap();
    let lock = dir.join(selfplay::LOCK);
    // This process is alive.
    std::fs::write(&lock, std::process::id().to_string()).unwrap();
    let refused = on_big_stack({
        let dir = dir.clone();
        move || try_run(&dir, 1, Govern::default(), true, 3, &options())
    });
    assert!(refused.unwrap_err().contains("in use by another self-play process"));
    // A process that has exited.
    let mut gone = std::process::Command::new(env!("CARGO_BIN_EXE_cantrip-selfplay")).arg("--help").stderr(std::process::Stdio::null()).spawn().unwrap();
    let pid = gone.id();
    gone.wait().unwrap();
    std::fs::write(&lock, pid.to_string()).unwrap();
    let report = run_big(&dir, 2, true, 3, options());
    assert_eq!(report["Games"], GAMES as u64);
    assert!(!lock.exists(), "the lock is let go at the end");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_binary_exits_3_when_the_governor_stops_it() {
    let dir = out("binary");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_cantrip-selfplay"))
        .args(["--model", &tiny().display().to_string(), "--out", &dir.display().to_string()])
        .args(["--backend", "tract", "--games", "500", "--lanes", "1", "--simulations", "8", "--max-turns", "2"])
        // A floor no machine has: stopped after its four readings (a second).
        .args(["--memory-floor", "1000000", "--memory-reserve", "0", "--commit-reserve", "0"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
    let report: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("selfplay-report.json")).unwrap()).unwrap();
    assert!(report["Stopped"]["Reason"].as_str().unwrap().contains("floor"), "{report}");
    assert_eq!(parts(&dir), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_stop_file_stops_the_binary_gracefully_and_one_there_at_the_start_plays_nothing() {
    let dir = out("stopfile");
    let run = |dir: &Path| {
        std::process::Command::new(env!("CARGO_BIN_EXE_cantrip-selfplay"))
            .args(["--model", &tiny().display().to_string(), "--out", &dir.display().to_string(), "--resume"])
            .args(["--backend", "tract", "--games", "500", "--lanes", "2", "--simulations", "8", "--max-turns", "2"])
            .args(["--shard-games", "2", "--memory-reserve", "0", "--commit-reserve", "0", "--host", "here"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut child = run(&dir);
    // Until the status file says a game is done (it is rewritten every few seconds).
    let status = dir.join(selfplay::STATUS);
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        let games = std::fs::read_to_string(&status).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v["Games"].as_u64());
        if games.is_some_and(|g| g >= 1) {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "no game in two minutes");
        std::thread::sleep(Duration::from_millis(200));
    }
    std::fs::write(dir.join(selfplay::STOP_FILE), "").unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(3));
    let report: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("selfplay-report.json")).unwrap()).unwrap();
    assert!(report["Stopped"]["Reason"].as_str().unwrap().starts_with("stop file"), "{report}");
    assert_eq!(report["Host"], "here");
    assert_eq!(parts(&dir), 0, "every shard closed");
    let played = games(&dir).len();
    assert!(played >= 1);
    let last: Value = serde_json::from_str(&std::fs::read_to_string(&status).unwrap()).unwrap();
    assert_eq!(last["Finished"], true);
    assert_eq!(last["Games"], played as u64);
    // Still there: a resume stops before its first game.
    assert_eq!(run(&dir).wait().unwrap().code(), Some(3));
    assert_eq!(games(&dir).len(), played, "no game played");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn games_split_by_grants_are_the_reference_games_each_once() {
    let reference = reference();
    let (a, b) = (out("grant-a"), out("grant-b"));
    let grants = out("grants");
    std::fs::create_dir_all(&grants).unwrap();
    // Out of order, and B's range between A's: each index granted to exactly one of them.
    std::fs::write(grants.join("a"), "4..6\n0\nend\n").unwrap();
    std::fs::write(grants.join("b"), "# b\n1..4\nend\n").unwrap();
    let ra = {
        let (a, g) = (a.clone(), grants.join("a"));
        on_big_stack(move || try_run_granted(&a, 2, Govern::default(), false, 2, &options(), Some(g)).unwrap())
    };
    let rb = {
        let (b, g) = (b.clone(), grants.join("b"));
        on_big_stack(move || try_run_granted(&b, 1, Govern::default(), false, 2, &options(), Some(g)).unwrap())
    };
    assert_eq!(ra["Games"], 3);
    assert_eq!(rb["Games"], 3);
    assert_eq!(ra["Grant"]["Granted"], 3);
    let (ga, gb_) = (games(&a), games(&b));
    assert_eq!(ga.keys().copied().collect::<Vec<_>>(), vec![7, 11, 12]);
    let mut all = ga.clone();
    for (seed, bytes) in gb_ {
        assert!(all.insert(seed, bytes).is_none(), "seed {seed} in both");
    }
    assert_eq!(all, reference, "the same games, byte for byte, as one run plays");
    for d in [a, b, grants] {
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn a_run_waits_for_its_grant_to_grow_until_it_says_end() {
    let dir = out("grant-wait");
    std::fs::create_dir_all(&dir).unwrap();
    let grant = dir.with_extension("grant");
    // A line not ended by a newline is not read yet.
    std::fs::write(&grant, "0..2\n2").unwrap();
    let handle = {
        let (dir, grant) = (dir.clone(), grant.clone());
        std::thread::Builder::new()
            .stack_size(cantrip_ai_host::STACK_SIZE)
            .spawn(move || try_run_granted(&dir, 2, Govern::default(), false, 5, &options(), Some(grant)).unwrap())
            .unwrap()
    };
    let status = dir.join(selfplay::STATUS);
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        let v = std::fs::read_to_string(&status).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
        if let Some(v) = v.filter(|v| v["Games"] == 2) {
            assert_eq!(v["Granted"], 2, "the cut line is not granted");
            break;
        }
        assert!(std::time::Instant::now() < deadline, "the two granted games were not played");
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(!handle.is_finished(), "the run waits while its grant is open");
    std::fs::write(&grant, "0..2\n2\nend\n").unwrap();
    let report = handle.join().unwrap();
    assert_eq!(report["Games"], 3);
    let reference = reference();
    let played = games(&dir);
    assert_eq!(played.keys().copied().collect::<Vec<_>>(), vec![7, 8, 9]);
    for (seed, bytes) in &played {
        assert_eq!(Some(bytes), reference.get(seed), "seed {seed}");
    }
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&grant);
}

#[test]
fn recover_closes_a_killed_runs_parts_and_plays_nothing() {
    let dir = out("recover");
    run_big(&dir, 1, false, 3, options());
    let first = shard::shards(&dir).unwrap()[0].clone();
    cut(&first, 5);
    assert_eq!(parts(&dir), 1);
    let run = Run {
        out: dir.clone(),
        games: GAMES,
        limit: None,
        lanes: 1,
        seed: 7,
        pairings: vec![pairing()],
        producer: "selfplay:tiny@8".into(),
        shard_mb: 64,
        shard_games: 3,
        resume: true,
        govern: Govern::default(),
        grant: None,
        host: None,
    };
    let r = selfplay::recover(&run, &options()).unwrap();
    assert_eq!(parts(&dir), 0);
    assert_eq!(recovered(&dir), 1);
    // The game the cut ended inside is gone; the rest are there, each once.
    assert_eq!(r["Games"], (GAMES - 1) as u64);
    assert_eq!(games(&dir).len(), GAMES - 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_grant_is_read_as_its_lines_say() {
    let (r, end) = selfplay::parse_grant("0..3\n# c\n\n7\nend\n5..6", 10).unwrap();
    assert_eq!(r, vec![0..3, 7..8]);
    assert!(end);
    assert!(selfplay::parse_grant("8..11\n", 10).is_err(), "past the games");
    assert!(selfplay::parse_grant("x\n", 10).is_err());
    let w = selfplay::Work::new(4, None);
    let picks: Vec<_> = (0..5).map(|_| w.take(|i| i == 2).unwrap()).collect();
    use selfplay::Pick::{Done, Game};
    assert_eq!(picks, vec![Game(0), Game(1), Game(3), Done, Done]);
}

#[test]
fn print_header_names_what_the_shards_will_carry_and_a_lapsed_lease_stops_the_binary() {
    let dir = out("header");
    let base = |dir: &Path| {
        let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_cantrip-selfplay"));
        c.args(["--model", &tiny().display().to_string(), "--out", &dir.display().to_string(), "--resume"])
            .args(["--backend", "tract", "--games", "500", "--lanes", "1", "--simulations", "8", "--max-turns", "2", "--seed", "7"])
            .args(["--shard-games", "1", "--memory-reserve", "0", "--commit-reserve", "0"]);
        c
    };
    let printed = base(&dir).arg("--print-header").output().unwrap();
    assert!(printed.status.success());
    let header: Value = serde_json::from_slice(&printed.stdout).unwrap();
    assert_eq!(header["seeds"], "7..507");
    assert!(header["producer"].as_str().unwrap().starts_with("selfplay:tiny:"));
    assert!(!dir.join("selfplay-run.json").exists(), "nothing played or written");
    // A lease that is never renewed: the run stops gracefully once it lapses, its finished games in closed shards.
    std::fs::create_dir_all(&dir).unwrap();
    let heartbeat = dir.join("heartbeat");
    std::fs::write(&heartbeat, "").unwrap();
    let status = base(&dir)
        .args(["--lease", &heartbeat.display().to_string(), "--lease-seconds", "3"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
    let report: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("selfplay-report.json")).unwrap()).unwrap();
    assert!(report["Stopped"]["Reason"].as_str().unwrap().contains("not renewed"), "{report}");
    assert_eq!(parts(&dir), 0);
    for path in shard::shards(&dir).unwrap() {
        let (h, _) = shard::read(&path).unwrap();
        assert_eq!(h.producer, header["producer"].as_str().unwrap());
        assert_eq!(h.text_hash, header["text_hash"].as_str().unwrap());
    }
    let _ = std::fs::remove_dir_all(&dir);
}
