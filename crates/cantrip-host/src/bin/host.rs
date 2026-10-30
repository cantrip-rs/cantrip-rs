//! The sidecar `Cantrip.Server` runs: any number of games, one JSON object per line on stdin, one reply per request on
//! stdout, in order; stderr is the log. The protocol is written up in docs/host-protocol.md, every payload
//! `protocol.rs`'s JSON, and `NativeRulesHost` (C#) drives this binary.
//!
//! With `--ai-record DIR` the host also records every game that ends as training samples for the AI player
//! (`cantrip_ai::record`), into zstd shards in DIR (`cantrip_ai::shard`), on worker threads: see `Recorder`. The
//! recorder is built only with the `ai-record` feature (`cargo build -p cantrip-host --features ai-record`), so the
//! shipping host compiles neither the AI player nor zstd; without it, `--ai-record` is refused.

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
use cantrip_engine::Game;
#[cfg(feature = "ai-record")]
use cantrip_obs::InfoMode;

use cantrip_host::protocol::{AnswerPayload, AttackMessage, BlockMessage, ViewHandle};
use cantrip_host::session::{Deal, Rejection, Session};
use cantrip_host::view::printed;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(tag = "Cmd")]
enum Request {
    NewGame {
        #[serde(rename = "Seats")]
        seats: Vec<SeatSpec>,
        #[serde(rename = "Seed")]
        seed: u64,
        #[serde(rename = "Shuffle", default = "yes")]
        shuffle: bool,
        #[serde(rename = "FirstPlayer", default)]
        first_player: Option<i32>,
        #[serde(rename = "Chooser", default)]
        chooser: Option<i32>,
        /// A Limited game's basic lands, which each sideboard holds any number of (`Setup::basic_lands_outside`).
        #[serde(rename = "BasicLandsOutside", default)]
        basic_lands_outside: Vec<String>,
        /// The format's minimum deck size, which the server knows from the table's format (`Setup::minimum_deck_size`, CR
        /// 100.2a-b): always said, never inferred. A scenario's is its own.
        #[serde(rename = "MinimumDeckSize", default)]
        minimum_deck_size: Option<u32>,
        /// A `.cantrip` scenario to play instead of a dealt game: its decks, order and dice are the scenario's, and the
        /// seats named in `Driven` are answered by the table, every other by the script (docs/scenarios-in-the-browser.md).
        #[serde(rename = "Scenario", default)]
        scenario: Option<ScenarioSpec>,
    },
    Start {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
    Answer {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Answer")]
        answer: AnswerPayload,
    },
    /// What the rule of an outstanding choice of cards says of a partial selection (docs/host-protocol.md, "Checking a
    /// selection"): nothing advances.
    Check {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Selected", default)]
        selected: Vec<ViewHandle>,
    },
    /// What the rules make of a declaration of blockers for an outstanding declaration (docs/host-protocol.md, "Checking a
    /// declaration of blockers"): nothing advances.
    CheckBlocks {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Blocks", default)]
        blocks: Vec<BlockMessage>,
    },
    /// What the rules make of a declaration of attackers for an outstanding declaration (docs/host-protocol.md, "Checking a
    /// declaration of attackers"): nothing advances.
    CheckAttacks {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
        #[serde(rename = "Attacks", default)]
        attacks: Vec<AttackMessage>,
    },
    Concede {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
    },
    /// The game ends from outside its rules: won by `Winner`'s seat, or a draw with none.
    End {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Winner", default)]
        winner: Option<i32>,
    },
    View {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "Seat")]
        seat: i32,
    },
    Oracle {
        #[serde(rename = "Name")]
        name: String,
    },
    /// Which cards with companion in a sideboard the deck fulfils the condition of (CR 702.139a), for the deck editor.
    Companions {
        #[serde(rename = "Deck")]
        deck: Vec<String>,
        #[serde(rename = "Sideboard", default)]
        sideboard: Vec<String>,
        #[serde(rename = "MinimumDeckSize")]
        minimum_deck_size: u32,
    },
    Close {
        #[serde(rename = "GameId")]
        game_id: u32,
    },
    /// A recording host only: a heuristic's value of the outstanding request's position for its seat (`Heuristic`, the
    /// tactical bot's model in log-odds), recorded with the game (`cantrip_ai::record::Dealt::heuristics`). Changes
    /// nothing in the game.
    #[cfg(feature = "ai-record")]
    Note {
        #[serde(rename = "GameId")]
        game_id: u32,
        #[serde(rename = "RequestId")]
        request_id: u32,
        #[serde(rename = "Heuristic")]
        heuristic: f32,
    },
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct ScenarioSpec {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Text")]
    text: String,
    #[serde(rename = "Driven", default)]
    driven: Vec<i32>,
}

#[derive(Deserialize)]
struct SeatSpec {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Deck")]
    deck: Vec<String>,
    /// The cards the seat owns outside the game (CR 100.4, 400.11a).
    #[serde(rename = "Sideboard", default)]
    sideboard: Vec<String>,
    /// The companion chosen as the deck was built (CR 702.139a): dealt into the sideboard beside it, and revealed before the
    /// game if the deck fulfils its condition (CR 103.2b). The server has refused a deck whose companion it doesn't.
    #[serde(rename = "Companion", default)]
    companion: Option<String>,
    /// The deck's name, where the table has one: the recorder's archetype label.
    #[serde(rename = "DeckName", default)]
    #[cfg_attr(not(feature = "ai-record"), allow(dead_code))]
    deck_name: Option<String>,
}

struct Host {
    games: HashMap<u32, Session>,
    next_game: u32,
    /// Set by `--ai-record`; `None` costs the game loop nothing.
    #[cfg(feature = "ai-record")]
    recorder: Option<Recorder>,
}

fn rejected(r: Rejection) -> Value {
    json!({ "Rejected": { "Code": r.code, "Detail": r.detail } })
}

fn ok(value: Value) -> Value {
    json!({ "Ok": value })
}

impl Host {
    fn handle(&mut self, request: Request) -> Value {
        match request {
            Request::NewGame { seats, seed, shuffle, first_player, chooser, basic_lands_outside, minimum_deck_size, scenario } => {
                #[cfg(feature = "ai-record")]
                let dealt = self.recorder.as_ref().map(|_| Dealt {
                    seed: if scenario.is_some() { 0 } else { seed },
                    names: seats.iter().map(|s| s.name.clone()).collect(),
                    decks: seats.iter().map(|s| s.deck_name.clone().unwrap_or_default()).collect(),
                    auto_start: scenario.is_none() && first_player.is_some() && chooser.is_none(),
                    ..Dealt::default()
                });
                let made = match scenario {
                    Some(spec) => Session::scenario(&spec.text, &spec.name, seats.into_iter().map(|s| s.name).collect(), &spec.driven),
                    None if minimum_deck_size.is_none() => {
                        Err(Rejection::new("InvalidSetup", "a dealt game says its format's minimum deck size (MinimumDeckSize)"))
                    }
                    None => {
                        let companions = seats.iter().map(|s| s.companion.clone()).collect();
                        let deal = Deal {
                            seats: seats.into_iter().map(|s| (s.name, s.deck, s.sideboard)).collect(),
                            basic_lands_outside,
                            companions,
                            minimum_deck_size: minimum_deck_size.unwrap_or_default(),
                            seed,
                            shuffle,
                            first_player,
                            chooser,
                        };
                        Session::new(deal)
                    }
                };
                match made {
                    Ok(session) => {
                        let id = self.next_game;
                        self.next_game += 1;
                        let decklists = session.decklists();
                        self.games.insert(id, session);
                        #[cfg(feature = "ai-record")]
                        if let (Some(recorder), Some(dealt)) = (self.recorder.as_mut(), dealt) {
                            recorder.dealt.insert(id, dealt);
                        }
                        ok(json!({ "GameId": id, "Decklists": decklists }))
                    }
                    Err(r) => rejected(r),
                }
            }
            Request::Start { game_id } => self.with(game_id, |g| Ok(g.step())),
            Request::Answer { game_id, request_id, seat, answer } => {
                let reply = self.with(game_id, |g| g.answer(request_id, seat, &answer));
                self.ended(game_id);
                reply
            }
            Request::Check { game_id, request_id, seat, selected } => self.with(game_id, |g| g.check(request_id, seat, &selected)),
            Request::CheckBlocks { game_id, request_id, seat, blocks } => self.with(game_id, |g| g.check_blocks(request_id, seat, &blocks)),
            Request::CheckAttacks { game_id, request_id, seat, attacks } => self.with(game_id, |g| g.check_attacks(request_id, seat, &attacks)),
            Request::Concede { game_id, seat } => {
                let reply = self.with(game_id, |g| g.concede(seat));
                self.ended(game_id);
                reply
            }
            Request::End { game_id, winner } => self.with(game_id, |g| g.end(winner)),
            Request::View { game_id, seat } => self.with(game_id, |g| g.view(seat).map(|v| json!({ "View": v }))),
            Request::Oracle { name } => {
                let card = cantrip_engine_card(&name);
                ok(json!({ "Card": card }))
            }
            Request::Companions { deck, sideboard, minimum_deck_size } => match cantrip_host::session::companions(&deck, &sideboard, minimum_deck_size) {
                Ok(checks) => ok(json!({ "Companions": checks })),
                Err(r) => rejected(r),
            },
            #[cfg(feature = "ai-record")]
            Request::Note { game_id, request_id, heuristic } => {
                if let (Some(session), Some(recorder)) = (self.games.get(&game_id), self.recorder.as_mut())
                    && let Some(dealt) = recorder.dealt.get_mut(&game_id)
                    && session.pending_request().is_some_and(|r| r.id == request_id)
                {
                    dealt.heuristics.insert(session.game().at, heuristic);
                }
                ok(json!({}))
            }
            Request::Close { game_id } => {
                let session = self.games.remove(&game_id);
                #[cfg(not(feature = "ai-record"))]
                drop(session);
                #[cfg(feature = "ai-record")]
                if let Some(session) = session
                    && let Some(recorder) = self.recorder.as_mut()
                    && let Some(dealt) = recorder.dealt.remove(&game_id)
                {
                    // Closed before it ended: the table's cap. Its samples are recorded with every result 0.
                    let game = session.game();
                    recorder.send(game.clone(), dealt, Ending::of(game, true));
                }
                ok(json!({}))
            }
        }
    }

    /// A recorded game that has just ended is handed to the recorder, once.
    #[cfg(feature = "ai-record")]
    fn ended(&mut self, game_id: u32) {
        let Some(recorder) = self.recorder.as_mut() else { return };
        if let Some(session) = self.games.get(&game_id)
            && session.is_over()
            && let Some(dealt) = recorder.dealt.remove(&game_id)
        {
            let game = session.game();
            recorder.send(game.clone(), dealt, Ending::of(game, false));
        }
    }

    /// Built without the recorder, nothing is recorded.
    #[cfg(not(feature = "ai-record"))]
    fn ended(&mut self, _game_id: u32) {}

    fn with(&mut self, game_id: u32, f: impl FnOnce(&mut Session) -> Result<Value, Rejection>) -> Value {
        match self.games.get_mut(&game_id) {
            Some(session) => match f(session) {
                Ok(v) => ok(v),
                Err(r) => rejected(r),
            },
            None => rejected(Rejection::new("TableClosed", format!("no game {game_id}"))),
        }
    }
}

fn cantrip_engine_card(name: &str) -> Option<cantrip_host::protocol::ObjectView> {
    let id = cantrip_engine::card(name)?;
    Some(printed(cantrip_engine::cards().get(id)))
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
}

#[cfg(feature = "ai-record")]
const USAGE: &str = "usage: cantrip-native-host [--ai-record DIR [--ai-record-mode full|decklist|hidden] [--ai-record-producer NAME] \
[--ai-record-seeds TEXT] [--ai-record-games N] [--ai-record-mb N] [--ai-record-threads N] [--ai-record-also MODE ...] [--ai-record-distill MODEL_DIR --ai-record-distill-server HOST:PORT [--ai-record-distill-worlds N]]]";

#[cfg(feature = "ai-record")]
fn config(args: &[String]) -> Result<Option<RecordConfig>, String> {
    let mut dir = None;
    let mut mode = InfoMode::Full;
    let mut producer = "unknown".to_string();
    let mut seeds = String::new();
    let (mut shard_games, mut shard_mb, mut threads) = (0usize, 64u64, 2usize);
    let mut also = Vec::new();
    let (mut distill_model, mut distill_server, mut distill_worlds) = (None, None, 8usize);
    let mut i = 0;
    while i < args.len() {
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
    Ok(dir.map(|dir| RecordConfig { dir, mode, producer, seeds, shard_games, shard_mb, threads, also, distill }))
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
type Job = (Game, Dealt, Ending);

/// What a recorder worker did.
#[cfg(feature = "ai-record")]
#[derive(Default)]
struct Totals {
    games: u64,
    samples: u64,
    bytes: u64,
    shards: usize,
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
                    work(&receive, &mut writer, mode, &mut also, evaluator.as_ref().map(|(e, w)| (e as &dyn cantrip_ai::Evaluator, *w)))
                })
                .expect("a recorder thread starts");
            workers.push(worker);
        }
        Ok(Recorder { send: Some(send), workers, dealt: HashMap::new(), config, started: std::time::Instant::now() })
    }

    fn send(&mut self, game: Game, dealt: Dealt, ending: Ending) {
        if let Some(send) = &self.send {
            let _ = send.send((game, dealt, ending));
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
) -> Totals {
    let prior = cantrip_ai::Prior::new();
    let mut totals = Totals::default();
    loop {
        let job = receive.lock().map(|r| r.recv());
        let Ok(Ok((game, dealt, ending))) = job else { break };
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
        games: HashMap::new(),
        next_game: 1,
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
        let reply = match serde_json::from_str::<Request>(&line) {
            Ok(request) => {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| host.handle(request)));
                result.unwrap_or_else(|_| rejected(Rejection::new("IllegalAnswer", "the engine failed on that request")))
            }
            Err(e) => rejected(Rejection::new("InvalidSetup", format!("unreadable request: {e}"))),
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
