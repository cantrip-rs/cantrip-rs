//! The sidecar `Cantrip.Server` runs: any number of games, one JSON object per line on stdin, one reply per request on
//! stdout, in order; stderr is the log. The protocol is written up in docs/host-protocol.md, every payload
//! `protocol.rs`'s JSON, and `NativeRulesHost` (C#) drives this binary.
//!
//! With `--ai-record DIR` the host also records every game that ends as training samples for the AI player
//! (`cantrip_ai::record`), into zstd shards in DIR (`cantrip_ai::shard`), on worker threads: see `Recorder`. The
//! recorder is built only with the `ai-record` feature (`cargo build -p cantrip-host --features ai-record`), so the
//! shipping host compiles neither the AI player nor zstd; without it, `--ai-record` is refused.

#[cfg(feature = "ai-record")]
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
#[cfg(feature = "ai-record")]
use std::path::PathBuf;
#[cfg(feature = "ai-record")]
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
#[cfg(feature = "ai-record")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "ai-record")]
use std::thread::JoinHandle;

#[cfg(feature = "ai-record")]
use cantrip_ai::record::{Dealt, Ending, Failures, record, table};
#[cfg(feature = "ai-record")]
use cantrip_ai::shard::{Header, ShardWriter, mode_name, parse_mode};
#[cfg(feature = "ai-record")]
use cantrip_ai::trace::{DealSpec, Trace};
#[cfg(feature = "ai-record")]
use cantrip_engine::Game;
#[cfg(feature = "ai-record")]
use cantrip_obs::InfoMode;

use cantrip_host::serve::{Request, Tables, guarded, parse};
use serde_json::Value;
#[cfg(feature = "ai-record")]
use serde_json::json;

struct Host {
    tables: Tables,
    /// Set by `--ai-record`; `None` costs the game loop nothing.
    #[cfg(feature = "ai-record")]
    recorder: Option<Recorder>,
}

impl Host {
    /// A request answered by the tables (`cantrip_host::serve`).
    #[cfg(not(feature = "ai-record"))]
    fn handle(&mut self, request: Request) -> Value {
        self.tables.handle(request)
    }

    /// A request answered by the tables, and what it deals, ends or notes recorded.
    #[cfg(feature = "ai-record")]
    fn handle(&mut self, request: Request) -> Value {
        match request {
            Request::NewGame { ref seats, seed, shuffle, first_player, chooser, ref basic_lands_outside, minimum_deck_size, commander, ref scenario, .. } => {
                let dealt = self.recorder.as_ref().map(|_| Dealt {
                    seed: if scenario.is_some() { 0 } else { seed },
                    names: seats.iter().map(|s| s.name.clone()).collect(),
                    decks: seats.iter().map(|s| s.deck_name.clone().unwrap_or_default()).collect(),
                    auto_start: scenario.is_none() && first_player.is_some() && chooser.is_none(),
                    ..Dealt::default()
                });
                // The game as what deals it again (`--ai-record-traces`): a dealt game's lists and setup, as the session
                // deals it. A scenario's game is not traced, nor is a variant's: a trace deals the format's start.
                let varied = seats.iter().any(|s| s.starting_life.is_some() || s.starting_hand_size.is_some() || s.maximum_hand_size.is_some() || !s.emblems.is_empty());
                let traced = self.recorder.as_ref().filter(|r| r.config.traces && scenario.is_none() && !varied).map(|_| {
                    let first = first_player.and_then(cantrip_host::view::player_at);
                    let opening = chooser.and_then(cantrip_host::view::player_at).or(first.filter(|_| !shuffle));
                    let mut t = Trace::new(
                        seed,
                        seats.iter().map(|s| s.deck_name.clone().unwrap_or_default()).collect(),
                        seats.iter().map(|s| s.deck.clone()).collect(),
                    );
                    t.deal = Some(DealSpec {
                        sideboards: seats.iter().map(|s| s.sideboard.clone()).collect(),
                        companions: seats.iter().map(|s| s.companion.clone()).collect(),
                        basic_lands_outside: basic_lands_outside.clone(),
                        minimum_deck_size: minimum_deck_size.unwrap_or_default(),
                        commander,
                        commanders: seats.iter().map(|s| s.commanders.clone()).collect(),
                        shuffle,
                        opening_chooser: opening.map(|p| p.0),
                    });
                    t
                });
                let reply = self.tables.handle(request);
                if let (Some(recorder), Some(dealt), Some(id)) = (self.recorder.as_mut(), dealt, reply["Ok"]["GameId"].as_u64()) {
                    let id = id as u32;
                    recorder.dealt.insert(id, dealt);
                    if let Some(t) = traced {
                        recorder.traces.insert(id, t);
                    }
                }
                reply
            }
            Request::Answer { game_id, .. } | Request::Concede { game_id, .. } => {
                let reply = self.tables.handle(request);
                self.ended(game_id);
                reply
            }
            Request::Note { game_id, request_id, heuristic } => {
                if let (Some(session), Some(recorder)) = (self.tables.get(game_id), self.recorder.as_mut())
                    && let Some(dealt) = recorder.dealt.get_mut(&game_id)
                    && session.pending_request().is_some_and(|r| r.id == request_id)
                {
                    dealt.heuristics.insert(session.game().at, heuristic);
                }
                cantrip_host::serve::ok(json!({}))
            }
            Request::Close { game_id } => {
                if let Some(session) = self.tables.remove(game_id)
                    && let Some(recorder) = self.recorder.as_mut()
                    && let Some(dealt) = recorder.dealt.remove(&game_id)
                {
                    // Closed before it ended: the table's cap. Its samples are recorded with every result 0.
                    let game = session.game();
                    let trace = recorder.traces.remove(&game_id);
                    recorder.send(game.clone(), dealt, Ending::of(game, true), trace);
                }
                cantrip_host::serve::ok(json!({}))
            }
            other => self.tables.handle(other),
        }
    }

    /// A recorded game that has just ended is handed to the recorder, once.
    #[cfg(feature = "ai-record")]
    fn ended(&mut self, game_id: u32) {
        let Some(recorder) = self.recorder.as_mut() else { return };
        if let Some(session) = self.tables.get(game_id)
            && session.is_over()
            && let Some(dealt) = recorder.dealt.remove(&game_id)
        {
            let game = session.game();
            let trace = recorder.traces.remove(&game_id);
            recorder.send(game.clone(), dealt, Ending::of(game, false), trace);
        }
    }
}

/// Card code recurses (resolution inside a settle inside a priority), so the host loop runs on a thread with room.
const STACK_SIZE: usize = 256 << 20;

/// `--ai-record DIR` and its options.
#[cfg(feature = "ai-record")]
struct RecordConfig {
    dir: PathBuf,
    mode: InfoMode,
    producer: String,
    seeds: String,
    /// Games a shard (0: no limit), and about how many megabytes compressed.
    shard_games: usize,
    shard_mb: u64,
    threads: usize,
    /// Modes each game is recorded in as well, each into `DIR/<mode>/`: the same game as its seats would have observed it
    /// in that mode (`--ai-record-also full`: the perfect-information positions beside the Decklist ones).
    also: Vec<InfoMode>,
    /// A perfect-information value to distil into the main mode's games (`cantrip_ai::distill`): its model directory,
    /// the inference server it is served by, and how many worlds a decision.
    distill: Option<(PathBuf, String, usize)>,
    /// Each dealt game's trace too, a JSON line each into `DIR/traces/` (`cantrip_ai::trace`), so that its positions can
    /// be stood in again (`cantrip-reanalyse`): `--ai-record-traces`, or `CANTRIP_AI_TRACES=1` where the table's command
    /// line cannot say it. A trace is written where it replays as the game up to where it ends (`Trace::replays_into`);
    /// one a payment by hand cut short (`Trace::of_game`) still holds the positions before it.
    traces: bool,
}

#[cfg(feature = "ai-record")]
const USAGE: &str = "usage: cantrip-native-host [--ai-record DIR [--ai-record-mode full|decklist|hidden] [--ai-record-producer NAME] \
[--ai-record-seeds TEXT] [--ai-record-games N] [--ai-record-mb N] [--ai-record-threads N] [--ai-record-also MODE ...] [--ai-record-distill MODEL_DIR --ai-record-distill-server HOST:PORT [--ai-record-distill-worlds N]] [--ai-record-traces]]";

#[cfg(feature = "ai-record")]
fn config(args: &[String]) -> Result<Option<RecordConfig>, String> {
    let mut dir = None;
    let mut mode = InfoMode::Full;
    let mut producer = "unknown".to_string();
    let mut seeds = String::new();
    let (mut shard_games, mut shard_mb, mut threads) = (0usize, 64u64, 2usize);
    let mut also = Vec::new();
    let (mut distill_model, mut distill_server, mut distill_worlds) = (None, None, 8usize);
    let mut traces = std::env::var("CANTRIP_AI_TRACES").is_ok_and(|v| v != "0" && !v.is_empty());
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--ai-record-traces" {
            traces = true;
            i += 1;
            continue;
        }
        let value = args.get(i + 1).cloned().ok_or(format!("{} needs a value", args[i]))?;
        let number = |v: &str| v.parse::<u64>().map_err(|_| format!("{} takes a number, not {v}", args[i]));
        match args[i].as_str() {
            "--ai-record" => dir = Some(PathBuf::from(value)),
            "--ai-record-mode" => mode = parse_mode(&value).ok_or(format!("no information mode {value}"))?,
            "--ai-record-producer" => producer = value,
            "--ai-record-seeds" => seeds = value,
            "--ai-record-games" => shard_games = number(&value)? as usize,
            "--ai-record-mb" => shard_mb = number(&value)?,
            "--ai-record-threads" => threads = (number(&value)? as usize).max(1),
            "--ai-record-also" => also.push(parse_mode(&value).ok_or(format!("no information mode {value}"))?),
            "--ai-record-distill" => distill_model = Some(PathBuf::from(value)),
            "--ai-record-distill-server" => distill_server = Some(value),
            "--ai-record-distill-worlds" => distill_worlds = (number(&value)? as usize).max(1),
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    let distill = match (distill_model, distill_server) {
        (Some(m), Some(s)) => Some((m, s, distill_worlds)),
        (None, None) => None,
        _ => return Err("--ai-record-distill needs --ai-record-distill-server, and the other way round".into()),
    };
    Ok(dir.map(|dir| RecordConfig { dir, mode, producer, seeds, shard_games, shard_mb, threads, also, distill, traces }))
}

/// Built without the recorder, the host takes no arguments: `--ai-record` asks for a build with it.
#[cfg(not(feature = "ai-record"))]
enum RecordConfig {}

#[cfg(not(feature = "ai-record"))]
const USAGE: &str = "usage: cantrip-native-host (built without the `ai-record` feature, which `--ai-record` needs)";

#[cfg(not(feature = "ai-record"))]
fn config(args: &[String]) -> Result<Option<RecordConfig>, String> {
    match args.first() {
        None => Ok(None),
        Some(a) if a.starts_with("--ai-record") => {
            Err(format!("{a}: this host was built without the recorder; build it with `cargo build -p cantrip-host --features ai-record`"))
        }
        Some(a) => Err(format!("unknown argument {a}")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match config(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cantrip-native-host: {e}\n{USAGE}");
            std::process::exit(2);
        }
    };
    let worker = std::thread::Builder::new()
        .name("cantrip-native-host".into())
        .stack_size(STACK_SIZE)
        .spawn(move || serve(config))
        .expect("the host thread starts");
    if worker.join().is_err() {
        std::process::exit(1);
    }
}

/// A finished game to record: a copy of it, how it was dealt and how it ended.
#[cfg(feature = "ai-record")]
type Job = (Game, Dealt, Ending, Option<Trace>);

/// What a recorder worker did.
#[cfg(feature = "ai-record")]
#[derive(Default)]
struct Totals {
    games: u64,
    samples: u64,
    bytes: u64,
    shards: usize,
    traces: u64,
    /// Of `traces`, those cut short (`Trace::complete` false).
    traces_cut: u64,
    failures: Failures,
}

/// Records ended games on worker threads, each writing shards of its own, so the table never waits on the walk or the
/// compression. Shards are closed (and so readable) when the host's stdin ends; a game not yet ended then is dropped.
#[cfg(feature = "ai-record")]
struct Recorder {
    send: Option<SyncSender<Job>>,
    workers: Vec<JoinHandle<Totals>>,
    /// The games being recorded, by game id, until they end.
    dealt: HashMap<u32, Dealt>,
    /// The traces of the games being recorded (`RecordConfig::traces`), as dealt, until they end.
    traces: HashMap<u32, Trace>,
    config: RecordConfig,
    started: std::time::Instant,
}

#[cfg(feature = "ai-record")]
impl Recorder {
    fn new(config: RecordConfig) -> io::Result<Recorder> {
        std::fs::create_dir_all(&config.dir)?;
        let header = Header::new(table(), &config.producer, &config.seeds, config.mode);
        let (send, receive) = sync_channel::<Job>(config.threads * 4);
        let receive = Arc::new(Mutex::new(receive));
        let mut workers = Vec::new();
        for i in 0..config.threads {
            let mut writer = ShardWriter::new(&config.dir, &format!("cantrip-w{i}"), header.clone(), config.shard_mb << 20, config.shard_games)?;
            let mut also = Vec::new();
            for &m in &config.also {
                let dir = config.dir.join(mode_name(m));
                std::fs::create_dir_all(&dir)?;
                let h = Header::new(table(), &config.producer, &config.seeds, m);
                also.push((m, ShardWriter::new(&dir, &format!("cantrip-w{i}"), h, config.shard_mb << 20, config.shard_games)?));
            }
            let traces = if config.traces {
                let dir = config.dir.join("traces");
                std::fs::create_dir_all(&dir)?;
                let path = dir.join(format!("cantrip-w{i}-{}.jsonl", std::process::id()));
                Some(io::BufWriter::new(std::fs::OpenOptions::new().create(true).append(true).open(path)?))
            } else {
                None
            };
            let receive = Arc::clone(&receive);
            let mode = config.mode;
            let distill = config.distill.clone();
            let worker = std::thread::Builder::new()
                .name(format!("cantrip-recorder-{i}"))
                .stack_size(STACK_SIZE)
                .spawn(move || {
                    // The distilling value's evaluator, a connection of this worker's own.
                    let evaluator = distill.as_ref().map(|(model, server, worlds)| {
                        (cantrip_ai::RemoteEvaluator::connect(server, model, table()).expect("the distilling model's server answers"), *worlds)
                    });
                    let mut traces = traces;
                    work(&receive, &mut writer, mode, &mut also, evaluator.as_ref().map(|(e, w)| (e as &dyn cantrip_ai::Evaluator, *w)), traces.as_mut())
                })
                .expect("a recorder thread starts");
            workers.push(worker);
        }
        Ok(Recorder { send: Some(send), workers, dealt: HashMap::new(), traces: HashMap::new(), config, started: std::time::Instant::now() })
    }

    fn send(&mut self, game: Game, dealt: Dealt, ending: Ending, trace: Option<Trace>) {
        if let Some(send) = &self.send {
            let _ = send.send((game, dealt, ending, trace));
        }
    }

    /// Records what is queued, closes the shards and reports, on stderr (which may be gone by now) and in DIR.
    fn finish(mut self) {
        drop(self.send.take());
        let mut all = Totals::default();
        for worker in self.workers.drain(..) {
            match worker.join() {
                Ok(t) => {
                    all.games += t.games;
                    all.samples += t.samples;
                    all.bytes += t.bytes;
                    all.shards += t.shards;
                    all.traces += t.traces;
                    all.traces_cut += t.traces_cut;
                    for (k, n) in t.failures {
                        *all.failures.entry(k).or_default() += n;
                    }
                }
                Err(_) => *all.failures.entry("recorder/worker panicked".into()).or_default() += 1,
            }
        }
        let report = json!({
            "Dir": self.config.dir.display().to_string(),
            "Mode": mode_name(self.config.mode),
            "Producer": self.config.producer,
            "Games": all.games,
            "Samples": all.samples,
            "Bytes": all.bytes,
            "Shards": all.shards,
            "Traces": all.traces,
            "TracesCutShort": all.traces_cut,
            "Unfinished": self.dealt.len(),
            "Seconds": self.started.elapsed().as_secs_f64(),
            "Failures": all.failures,
        });
        let mut err = io::stderr();
        let _ = writeln!(err, "cantrip-native-host: recorded {} games, {} samples, {} bytes in {} shards; {} unfinished dropped", all.games, all.samples, all.bytes, all.shards, self.dealt.len());
        for (k, n) in &all.failures {
            let _ = writeln!(err, "cantrip-native-host: not recorded: {n} x {k}");
        }
        let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let path = self.config.dir.join(format!("report-{}-{millis}.json", std::process::id()));
        let _ = std::fs::write(path, serde_json::to_string_pretty(&report).unwrap_or_default());
    }
}

#[cfg(feature = "ai-record")]
fn work(
    receive: &Mutex<Receiver<Job>>,
    writer: &mut ShardWriter,
    mode: InfoMode,
    also: &mut [(InfoMode, ShardWriter)],
    distill: Option<(&dyn cantrip_ai::Evaluator, usize)>,
    mut traces: Option<&mut io::BufWriter<std::fs::File>>,
) -> Totals {
    let prior = cantrip_ai::Prior::new();
    let mut totals = Totals::default();
    loop {
        let job = receive.lock().map(|r| r.recv());
        let Ok(Ok((game, dealt, ending, trace))) = job else { break };
        // The trace first, from the game as it stands: kept only where it replays as the game up to where it ends.
        if let (Some(base), Some(w)) = (trace, traces.as_deref_mut()) {
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let t = Trace::of_game(base, &game);
                let ok = t.replays_into(&game);
                (t, ok)
            }));
            match made {
                Ok((t, true)) => match serde_json::to_string(&t) {
                    Ok(line) if writeln!(w, "{line}").and_then(|_| w.flush()).is_ok() => {
                        totals.traces += 1;
                        totals.traces_cut += u64::from(!t.complete);
                    }
                    _ => *totals.failures.entry("recorder/trace write failed".into()).or_default() += 1,
                },
                Ok(_) => *totals.failures.entry("recorder/trace does not replay".into()).or_default() += 1,
                Err(_) => *totals.failures.entry("recorder/trace panicked".into()).or_default() += 1,
            }
        }
        // The other modes first, from copies: the main record walks the game itself.
        for (m, w) in also.iter_mut() {
            let copy = game.clone();
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| record(copy, &dealt, &ending, *m, table())));
            match made {
                Ok((g, _)) => {
                    if w.write(&g).is_err() {
                        *totals.failures.entry(format!("recorder/{} shard write failed", mode_name(*m))).or_default() += 1;
                    }
                }
                Err(_) => *totals.failures.entry(format!("recorder/{} game walk panicked", mode_name(*m))).or_default() += 1,
            }
        }
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match distill {
            Some((evaluator, worlds)) => {
                let seed = dealt.seed;
                let d = |g: &Game, seat: cantrip_engine::PlayerId, k: &cantrip_ai::Knowledge| {
                    let at = g.state.mark().0 as u64;
                    cantrip_ai::distill::world_mean(g, seat, k, table(), evaluator, &prior, worlds, seed ^ (at << 20))
                };
                cantrip_ai::record::record_distilled(game, &dealt, &ending, mode, table(), None, Some(&d))
            }
            None => record(game, &dealt, &ending, mode, table()),
        }));
        match made {
            Ok((game, failures)) => {
                for (k, n) in failures {
                    *totals.failures.entry(k).or_default() += n;
                }
                if writer.write(&game).is_err() {
                    *totals.failures.entry("recorder/shard write failed".into()).or_default() += 1;
                }
            }
            Err(_) => *totals.failures.entry("recorder/game walk panicked".into()).or_default() += 1,
        }
    }
    if writer.close().is_err() {
        *totals.failures.entry("recorder/shard close failed".into()).or_default() += 1;
    }
    for (_, w) in also.iter_mut() {
        if w.close().is_err() {
            *totals.failures.entry("recorder/shard close failed".into()).or_default() += 1;
        }
    }
    totals.games = writer.games;
    totals.samples = writer.samples;
    totals.bytes = writer.bytes;
    totals.shards = writer.written.len();
    totals
}

fn serve(config: Option<RecordConfig>) {
    eprintln!("cantrip-native-host: {} cards loaded", cantrip_engine::cards().cards.len());
    #[cfg(not(feature = "ai-record"))]
    let _ = config;
    #[cfg(feature = "ai-record")]
    let recorder = config.map(|c| {
        let dir = c.dir.display().to_string();
        let mode = mode_name(c.mode);
        let r = Recorder::new(c).unwrap_or_else(|e| panic!("cannot record into {dir}: {e}"));
        eprintln!("cantrip-native-host: recording {mode} observations into {dir}");
        r
    });
    let mut host = Host {
        tables: Tables::default(),
        #[cfg(feature = "ai-record")]
        recorder,
    };
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match parse(&line) {
            Ok(request) => guarded(|| host.handle(request)),
            Err(refused) => refused,
        };
        let text = serde_json::to_string(&reply).expect("a reply serialises");
        if writeln!(out, "{text}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
    #[cfg(feature = "ai-record")]
    if let Some(recorder) = host.recorder.take() {
        recorder.finish();
    }
}
