//! Self-play for expert iteration (docs/ai-player-plan.md, "Phase 4"): both seats of a game search with one model at a
//! fixed number of simulations, in one information mode, and each finished game is recorded with the search's root
//! visits as the policy target of every sub-step it searched (`cantrip_ai::record::record_searched`).
//!
//! It runs in-process, on lanes of its own (a thread, an evaluator and a shard writer each), rather than as a table
//! with seats in an AI host beside it: a game is the engine's `Game` and the search's `Decision` directly, with no
//! mirror, no JSON and no handles between them, and the visits never leave the process that made them.
//!
//! A seat's first `sampled` searched decisions in a game are sampled from the root's visits at `temperature` (the
//! search's `Choice::Sample`), the rest the most visited. A game past `max_turns` or `max_answers` is closed as a cap
//! (every result 0) and counted. An answer the search could not make is the network's alone; a game where neither
//! answers is dropped and counted.
//!
//! A resumed run (`Run::resume`) reads the closed shards already in `out` and skips the games whose seeds they hold:
//! game `i` is always dealt from `seed + i`, and its pairing, seats and first player come from `i`, so the rest of the
//! schedule is unchanged. A `.part` shard that reads whole is recovered and renamed; one that ends inside a game is
//! ignored, and its games are played again.

use std::collections::{BTreeMap, HashSet};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cantrip_ai::net::{self, Choice, Payment};
use cantrip_ai::record::{Dealt, Ending, Failures, Root, Roots, record_searched, table};
use cantrip_ai::search::{self, Config, Fusion, Shape, WallClock};
use cantrip_ai::shard::{
    GameRecord, Header, ShardWriter, mode_name, read as read_shard, shards as shard_files,
};
use cantrip_ai::trace::{Seats, Trace};
use cantrip_ai::{Evaluator, Knowledge, Prior};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use serde_json::{Value, json};

use crate::STACK_SIZE;

/// Two decklists played against each other, by name.
#[derive(Clone, Debug)]
pub struct Pairing {
    pub names: [String; 2],
    pub lists: [Vec<String>; 2],
}

/// A decklist file of `decks/`: `N Name` lines up to a sideboard, `#` comments.
pub fn load_deck(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut cards = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .take_while(|l| !l.eq_ignore_ascii_case("sideboard"))
    {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((n, name)) = line.split_once(' ') else {
            continue;
        };
        let Ok(n) = n.parse::<usize>() else { continue };
        if card(name.trim()).is_none() {
            return Err(format!(
                "{}: no card is named \"{}\"",
                path.display(),
                name.trim()
            ));
        }
        cards.extend(std::iter::repeat_n(name.trim().to_string(), n));
    }
    Ok(cards)
}

/// How the games are played.
#[derive(Clone, Debug)]
pub struct Options {
    /// The model's name, as the seats and the shards' producer say it.
    pub model: String,
    pub simulations: usize,
    /// A seat's searched decisions sampled from the visits, at the start of each game.
    pub sampled: u32,
    pub temperature: f32,
    pub mode: InfoMode,
    /// Worlds a Decklist or Hidden search deals.
    pub worlds: usize,
    pub max_turns: u32,
    pub max_answers: u32,
    /// Modes each game is recorded in as well, each into `OUT/<mode>/`: the same game as the seat in that mode would
    /// have observed it, labels and visit targets included (the true hidden state is a label in every mode).
    pub also: Vec<InfoMode>,
    /// Whether each game's trace is written too, a JSON line each into `OUT/traces/`.
    pub traces: bool,
    /// Leaves a search sends to the network at once (`search::Config::batch`).
    pub batch: usize,
    /// Playout-cap randomisation (KataGo): a share `full_share` of searched decisions get `full_simulations`, and only
    /// those are policy targets; the rest get `simulations` (the fast budget), and are value targets alone. Off with
    /// `full_simulations` 0. Tree reuse carries a full search's visits into the fast searches after it.
    pub full_simulations: usize,
    pub full_share: f32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            model: "model".into(),
            simulations: 64,
            sampled: 20,
            temperature: 1.0,
            mode: InfoMode::Full,
            worlds: 8,
            max_turns: 60,
            max_answers: 6000,
            also: Vec::new(),
            traces: false,
            batch: 8,
            full_simulations: 0,
            full_share: 0.0,
        }
    }
}

/// One game played: the game, how it was dealt and ended, what the search made of each searched request, and counts.
pub struct Played {
    pub game: Game,
    pub dealt: Dealt,
    pub ending: Ending,
    pub roots: Roots,
    pub answers: u32,
    /// Requests searched with a choice (root visits recorded), and the simulations and evaluations they took.
    pub searched: u32,
    pub simulations: u64,
    pub evaluations: u64,
    /// Answers that were the network's alone because the search failed.
    pub network_fallbacks: u32,
    /// The game as what deals and answers it again (`cantrip_ai::trace`), when `Options::traces` asks for it.
    pub trace: Option<Trace>,
    /// The search's tree at each searched request.
    pub shapes: Vec<Shape>,
    /// The worlds' agreement at the root of each searched request that dealt two or more.
    pub fusions: Vec<Fusion>,
    pub elapsed: Duration,
    /// Where the answering time went (`Timing`).
    pub timing: Timing,
    /// Searched decisions given the full budget of playout-cap randomisation.
    pub full_searched: u32,
}

/// Where a game's answering time went, in seconds: all answers (searched or not), the searches, and of those the
/// encoding of leaves and the network's evaluations (from the request sent to the reply received: the inference
/// server's time and the round trip). A search's time less those two is the engine's and the tree's.
#[derive(Clone, Copy, Debug, Default)]
pub struct Timing {
    pub answering: f64,
    pub searching: f64,
    pub encoding: f64,
    pub evaluating: f64,
    /// Of the searches' lines (`search::Searched::play_time`): all, copying state, moving, observing.
    pub play: f64,
    pub setup: f64,
    pub goto: f64,
    pub observe: f64,
}

impl Timing {
    fn add(&mut self, o: &Timing) {
        self.answering += o.answering;
        self.searching += o.searching;
        self.encoding += o.encoding;
        self.evaluating += o.evaluating;
        self.play += o.play;
        self.setup += o.setup;
        self.goto += o.goto;
        self.observe += o.observe;
    }
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Plays one game of `pairing` dealt from `seed`, `first` going first, both seats searching with `evaluator`.
pub fn play(
    evaluator: &dyn Evaluator,
    pairing: &Pairing,
    seed: u64,
    first: usize,
    options: &Options,
    prior: &Prior,
) -> Result<Played, String> {
    let started = Instant::now();
    let decks = vec![pairing.lists[0].clone(), pairing.lists[1].clone()];
    let mut game = deal(&decks, seed, true, None)?;
    let seats = 2;
    let mut knowledge: Vec<Knowledge> = (0..seats)
        .map(|i| Knowledge::new(PlayerId(i as u8)))
        .collect();
    let mut payments: Vec<Payment> = vec![Payment::default(); seats];
    // Each seat's search tree, kept for its next request (`search::Config::reuse`, on by default).
    let mut trees: Vec<Option<search::Kept>> = (0..seats).map(|_| None).collect();
    let mut searched_by = vec![0u32; seats];
    let mut roots = Roots::new();
    let mut shapes = Vec::new();
    let mut timing = Timing::default();
    let mut full_searched = 0u32;
    let mut fusions = Vec::new();
    let (mut answers, mut searched, mut simulations, mut evaluations, mut network_fallbacks) =
        (0u32, 0u32, 0u64, 0u64, 0u32);
    let mut capped = false;
    let mut trace = options.traces.then(|| (Trace::new(seed, pairing.names.to_vec(), decks.clone()), Seats::new(&game)));
    while !game.is_over() {
        let Some(request) = game.pending().cloned() else {
            break;
        };
        if game.state.turn.number > options.max_turns || answers >= options.max_answers {
            capped = true;
            break;
        }
        answers += 1;
        if matches!(request.kind, RequestKind::ChooseStartingPlayer { .. }) {
            let answer = Answer::Player(PlayerId(first as u8));
            if let Some((t, seats)) = &mut trace {
                t.keep(seats, &game, &request, &answer, None, None);
            }
            game.answer(
                request.id,
                request.player,
                answer,
            )
            .map_err(|Rejected(why)| format!("who goes first: {why}"))?;
            continue;
        }
        let p = request.player.index();
        knowledge[p].update(&game);
        let at = game.at;
        let choice = if searched_by[p] < options.sampled {
            Choice::Sample {
                temperature: options.temperature,
                seed: splitmix(seed ^ ((answers as u64) << 20) ^ 0x5e1f),
            }
        } else {
            Choice::Best
        };
        // Playout-cap randomisation: this decision's budget, full or fast, by a draw of its own.
        let capped_off = options.full_simulations == 0;
        let full = capped_off
            || (splitmix(seed ^ ((request.id as u64) << 28) ^ 0xf011) >> 11) as f64 / (1u64 << 53) as f64 <= options.full_share as f64;
        let budget = if !capped_off && full { options.full_simulations } else { options.simulations };
        let config = Config {
            choice,
            seed: splitmix(seed ^ ((request.id as u64) << 24) ^ 0x5ea2c4),
            worlds: options.worlds,
            batch: options.batch.max(1),
            ..Config::simulations(budget)
        };
        let k = &knowledge[p];
        let tree = &mut trees[p];
        let asked = Instant::now();
        let decided = payments[p].decide(&game, &request, k, options.mode, |request, guard| {
            search::search_keeping(
                &game,
                request,
                k,
                options.mode,
                table(),
                evaluator,
                &config,
                prior,
                guard,
                &WallClock,
                tree,
            )
        });
        let decided = match decided {
            Ok(d) => d,
            Err(_) => {
                network_fallbacks += 1;
                let request = payments[p].filtered(&game, &request);
                net::answer(
                    &game,
                    &request,
                    k,
                    options.mode,
                    table(),
                    evaluator,
                    Choice::Best,
                )
                .map_err(|e| {
                    format!(
                        "turn {}: neither the search nor the network answered: {}",
                        game.state.turn.number, e.0
                    )
                })?
            }
        };
        timing.answering += asked.elapsed().as_secs_f64();
        if let Some(s) = &decided.search {
            timing.searching += s.elapsed.as_secs_f64();
            timing.encoding += decided.encode.as_secs_f64();
            timing.evaluating += decided.evaluate.as_secs_f64();
            timing.play += s.play_time.as_secs_f64();
            timing.setup += s.setup_time.as_secs_f64();
            timing.goto += s.goto_time.as_secs_f64();
            timing.observe += s.observe_time.as_secs_f64();
        }
        if let Some(s) = &decided.search {
            simulations += s.simulations as u64;
            evaluations += s.evaluations as u64;
            if !s.visits.is_empty() {
                searched += 1;
                searched_by[p] += 1;
                full_searched += u32::from(full && !capped_off);
                // The tree's shape of the searches that are policy targets.
                if full {
                    shapes.push(s.shape);
                }
                fusions.extend(s.fusion);
                roots.insert(
                    at,
                    Root {
                        visits: s.visits.clone(),
                        value: s.root_value,
                        policy: full,
                    },
                );
            }
        }
        let mut kept = trace.as_ref().map(|(t, _)| (t.steps.len(), t.complete));
        if let Some((t, seats)) = &mut trace {
            let searched = decided.search.as_ref().filter(|s| !s.visits.is_empty()).map(|s| (s.root_value, s.net_value));
            t.keep(seats, &game, &request, &decided.answer, Some(&decided.factored), searched);
        }
        // Noted before it is given, while the objects it names stand; taken back if the engine refuses it.
        let board = cantrip_ai::cancel::board_hash(&game.state);
        let before = payments[p].clone();
        payments[p].played(&game, board, &request, &decided.answer);
        if let Err(Rejected(why)) = game.answer(request.id, request.player, decided.answer.clone())
        {
            payments[p] = before;
            // The answer kept was not given: the network's is.
            if let (Some((t, _)), Some((n, complete))) = (&mut trace, kept.take()) {
                t.steps.truncate(n);
                t.complete = complete;
            }
            // The search's answers are the engine's to accept (tests/search.rs); one refused is the network's turn.
            network_fallbacks += 1;
            roots.remove(&at);
            let filtered = payments[p].filtered(&game, &request);
            let d = net::answer(
                &game,
                &filtered,
                &knowledge[p],
                options.mode,
                table(),
                evaluator,
                Choice::Best,
            )
            .map_err(|e| format!("refused ({why}), and the network: {}", e.0))?;
            if let Some((t, seats)) = &mut trace {
                t.keep(seats, &game, &request, &d.answer, Some(&d.factored), None);
            }
            payments[p].played(&game, board, &request, &d.answer);
            game.answer(request.id, request.player, d.answer.clone())
                .map_err(|Rejected(why2)| format!("refused twice: {why}; {why2}"))?;
        }
    }
    let dealt = Dealt {
        seed,
        names: (1..=seats)
            .map(|i| format!("{}{i}", options.model))
            .collect(),
        decks: pairing.names.to_vec(),
        auto_start: true,
        ..Dealt::default()
    };
    let ending = Ending::of(&game, capped || !game.is_over());
    let trace = trace.map(|(mut t, _)| {
        t.outcome = (0..seats)
            .map(|p| match ending.outcome {
                _ if ending.capped => 0,
                Outcome::Won(w) if w.index() == p => 1,
                Outcome::Won(_) => -1,
                _ => 0,
            })
            .collect();
        t
    });
    Ok(Played {
        game,
        dealt,
        ending,
        roots,
        answers,
        searched,
        simulations,
        evaluations,
        network_fallbacks,
        trace,
        shapes,
        fusions,
        elapsed: started.elapsed(),
        timing,
        full_searched,
    })
}

/// A `Shape`'s measures, by the names the report gives them.
fn shape_measures(s: &Shape) -> [(&'static str, u64); 8] {
    [
        ("RootOptions", s.root_options as u64),
        ("RootVisited", s.root_visited as u64),
        ("DeepestSteps", s.deepest_steps as u64),
        ("DeepestDecisions", s.deepest_decisions as u64),
        ("DeepestTurns", u64::from(s.deepest_turns)),
        ("LineSteps", s.line_steps as u64),
        ("LineDecisions", s.line_decisions as u64),
        ("LineTurns", u64::from(s.line_turns)),
    ]
}

/// The value at quantile `q` of a histogram.
fn quantile(h: &BTreeMap<u64, u64>, q: f64) -> u64 {
    let total: u64 = h.values().sum();
    let mut seen = 0;
    for (v, n) in h {
        seen += n;
        if seen as f64 >= q * total as f64 {
            return *v;
        }
    }
    0
}

/// The record of a game played.
pub fn record(played: Played, mode: InfoMode) -> (GameRecord, Failures) {
    let Played {
        game,
        dealt,
        ending,
        roots,
        ..
    } = played;
    record_searched(game, &dealt, &ending, mode, table(), Some(&roots))
}

/// A run of self-play: where the games go and how many.
#[derive(Clone, Debug)]
pub struct Run {
    pub out: PathBuf,
    pub games: usize,
    /// Stop dealing new games after this long.
    pub limit: Option<Duration>,
    pub lanes: usize,
    pub seed: u64,
    pub pairings: Vec<Pairing>,
    /// The producer the shards' header names.
    pub producer: String,
    pub shard_mb: u64,
    pub shard_games: usize,
    /// Continue a run already in `out`: games whose seeds are in its closed shards are not played again.
    pub resume: bool,
}

/// Game `i`'s pairing, seats and first player: the pairings in turn, each dealt with the decks either way round and
/// either seat first.
pub fn schedule(i: usize, pairings: usize) -> (usize, bool, usize) {
    let p = i % pairings;
    let swap = (i / pairings) % 2 == 1;
    let first = (i / (2 * pairings)) % 2;
    (p, swap, first)
}

/// The finished shards and the `.part` files an interrupted writer left, sorted. A `.part` that reads whole is a
/// finished shard that was never renamed; one that ends inside a game is ignored.
fn shard_paths(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut all = shard_files(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let parts = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| {
            let path = e.ok()?.path();
            path.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".part"))
                .then_some(path)
        });
    all.extend(parts);
    all.sort();
    Ok(all)
}

#[derive(Default)]
struct Totals {
    games: u64,
    samples: u64,
    capped: u64,
    dropped: u64,
    answers: u64,
    searched: u64,
    simulations: u64,
    evaluations: u64,
    network_fallbacks: u64,
    turns: u64,
    visit_samples: u64,
    game_seconds: f64,
    failures: Failures,
    errors: BTreeMap<String, u64>,
    /// Each `Shape` measure's histogram over the searched requests.
    shape: BTreeMap<&'static str, BTreeMap<u64, u64>>,
    /// Searched requests over two or more worlds, those whose worlds' most visited root options differed, the worlds
    /// over them, and the worlds that went with the plurality.
    fusion_roots: u64,
    fusion_split: u64,
    fusion_worlds: u64,
    fusion_plurality: u64,
    timing: Timing,
    full_searched: u64,
    /// Wins by deck name, and games by deck name.
    wins: BTreeMap<String, u64>,
    seats: BTreeMap<String, u64>,
    first_wins: u64,
    decided: u64,
}

/// What the shards in `out` already hold: the game indices finished, and the totals those games make. A finished
/// `.shard.zst` and a `.part` that reads whole are both read; a `.part` that ends inside a game is ignored.
fn resumed(out: &Path, run: &Run, options: &Options) -> Result<(HashSet<usize>, Totals), String> {
    let mut done = HashSet::new();
    let mut t = Totals::default();
    if !out.exists() {
        return Ok((done, t));
    }
    let seeds = format!("{}..{}", run.seed, run.seed + run.games as u64);
    for path in shard_paths(out)? {
        let partial = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with(".part"));
        let (header, games) = match read_shard(&path) {
            Ok(read) => read,
            Err(_) if partial => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        if partial {
            let final_path = path.with_extension("");
            let _ = std::fs::rename(&path, &final_path);
        }
        if header.producer != run.producer
            || header.mode != mode_name(options.mode)
            || header.text_hash != table().hash
            || header.seeds != seeds
        {
            return Err(format!(
                "{} is from another self-play run (producer {}, mode {}, seeds {}); remove {} or resume the run it belongs to",
                path.display(),
                header.producer,
                header.mode,
                header.seeds,
                out.display()
            ));
        }
        for game in games {
            let index = game
                .meta
                .seed
                .checked_sub(run.seed)
                .filter(|&i| i < run.games as u64)
                .ok_or_else(|| {
                    format!(
                        "{}: game seed {} is not in {}..{}",
                        path.display(),
                        game.meta.seed,
                        run.seed,
                        run.seed + run.games as u64
                    )
                })? as usize;
            if !done.insert(index) {
                continue;
            }
            t.games += 1;
            t.samples += game.samples.len() as u64;
            t.visit_samples += game.samples.iter().filter(|s| s.visits.len() > 1).count() as u64;
            t.capped += (game.meta.ended == "cap") as u64;
            t.answers += game.meta.answers as u64;
            t.turns += game.meta.turns as u64;
            let decks = if game.meta.decks.is_empty() {
                &game.meta.names
            } else {
                &game.meta.decks
            };
            for name in decks {
                *t.seats.entry(name.clone()).or_default() += 1;
            }
            if let Some(w) = game.meta.winner.filter(|_| game.meta.ended != "cap")
                && let Some(name) = decks.get(w as usize - 1)
            {
                *t.wins.entry(name.clone()).or_default() += 1;
                t.decided += 1;
                let first = schedule(index, run.pairings.len()).2;
                t.first_wins += (w as usize - 1 == first) as u64;
            }
        }
    }
    Ok((done, t))
}

/// Plays `run.games` games on `run.lanes` lanes, each with its own evaluator from `load`, and writes their shards and a
/// report (`selfplay-report.json`) into `run.out`. Returns the report.
pub fn run<F>(run: &Run, options: &Options, prior: &Prior, load: F) -> Result<Value, String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    std::fs::create_dir_all(&run.out).map_err(|e| format!("{}: {e}", run.out.display()))?;
    if !run.resume && !shard_paths(&run.out)?.is_empty() {
        return Err(format!(
            "{} already has shards; pass --resume or remove them",
            run.out.display()
        ));
    }
    let identity = json!({
        "producer": run.producer,
        "mode": mode_name(options.mode),
        "seed": run.seed,
        "games": run.games,
        "simulations": options.simulations,
        "sampled": options.sampled,
        "temperature": options.temperature,
        "worlds": options.worlds,
        "max_turns": options.max_turns,
        "max_answers": options.max_answers,
        "also": options.also.iter().map(|&m| mode_name(m)).collect::<Vec<_>>(),
        "traces": options.traces,
        "pairings": run.pairings.iter().map(|p| format!("{}/{}", p.names[0], p.names[1])).collect::<Vec<_>>(),
    });
    let manifest = run.out.join("selfplay-run.json");
    if manifest.exists() && run.resume {
        let old: Value = serde_json::from_str(
            &std::fs::read_to_string(&manifest)
                .map_err(|e| format!("{}: {e}", manifest.display()))?,
        )
        .map_err(|e| format!("{}: {e}", manifest.display()))?;
        if old != identity {
            return Err(format!(
                "{} is from another self-play run; remove {} or resume with the same settings",
                manifest.display(),
                run.out.display()
            ));
        }
    } else if !manifest.exists() || !run.resume {
        let _ = std::fs::write(
            &manifest,
            serde_json::to_string_pretty(&identity).unwrap_or_default(),
        );
    }
    // A bar on a terminal or when the driver asks for one; plain lines otherwise, for a log.
    let progress = std::env::var("CANTRIP_PROGRESS")
        .map_or_else(|_| std::io::stderr().is_terminal(), |v| v != "0");
    let header = Header::new(
        table(),
        &run.producer,
        &format!("{}..{}", run.seed, run.seed + run.games as u64),
        options.mode,
    );
    let (completed, resumed_totals) = if run.resume {
        resumed(&run.out, run, options)?
    } else {
        (HashSet::new(), Totals::default())
    };
    let resumed_games = resumed_totals.games;
    let (resumed_samples, resumed_visits) = (resumed_totals.samples, resumed_totals.visit_samples);
    let next = AtomicUsize::new(0);
    let totals = Mutex::new(resumed_totals);
    let started = Instant::now();
    let load = Arc::new(load);
    std::thread::scope(|scope| -> Result<(), String> {
        let mut lanes = Vec::new();
        for lane in 0..run.lanes.max(1) {
            let header = header.clone();
            let (next, totals, load, completed) = (&next, &totals, Arc::clone(&load), &completed);
            let worker = std::thread::Builder::new().name(format!("selfplay-{lane}")).stack_size(STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                let evaluator = load()?;
                let mut writer = ShardWriter::new(&run.out, &format!("selfplay-l{lane}"), header.clone(), run.shard_mb << 20, run.shard_games).map_err(|e| e.to_string())?;
                let mut also = options
                    .also
                    .iter()
                    .map(|&mode| {
                        let header = Header { mode: mode_name(mode).to_string(), ..header.clone() };
                        ShardWriter::new(&run.out.join(mode_name(mode)), &format!("selfplay-l{lane}"), header, run.shard_mb << 20, run.shard_games).map(|w| (mode, w))
                    })
                    .collect::<std::io::Result<Vec<_>>>()
                    .map_err(|e| e.to_string())?;
                // A JSON line a game; a resumed run appends, and a reader keeps the last line of a seed.
                let mut traces = if options.traces {
                    let dir = run.out.join("traces");
                    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                    let path = dir.join(format!("selfplay-l{lane}-{}.jsonl", std::process::id()));
                    Some(std::io::BufWriter::new(std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("{}: {e}", path.display()))?))
                } else {
                    None
                };
                loop {
                    if run.limit.is_some_and(|l| started.elapsed() >= l) {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= run.games {
                        break;
                    }
                    if completed.contains(&i) {
                        continue;
                    }
                    let (p, swap, first) = schedule(i, run.pairings.len());
                    let mut pairing = run.pairings[p].clone();
                    if swap {
                        pairing.names.swap(0, 1);
                        pairing.lists.swap(0, 1);
                    }
                    let seed = run.seed + i as u64;
                    match play(evaluator.as_ref(), &pairing, seed, first, options, prior) {
                        Ok(mut played) => {
                            let (answers, searched, sims, evals, fallbacks, secs) =
                                (played.answers, played.searched, played.simulations, played.evaluations, played.network_fallbacks, played.elapsed.as_secs_f64());
                            let (turns, capped) = (played.game.state.turn.number, played.ending.capped);
                            if let (Some(w), Some(t)) = (&mut traces, played.trace.take()) {
                                let line = serde_json::to_string(&t).map_err(|e| e.to_string())?;
                                writeln!(w, "{line}").and_then(|_| w.flush()).map_err(|e| e.to_string())?;
                            }
                            let shapes = std::mem::take(&mut played.shapes);
                            let timing = played.timing;
                            let full_searched = played.full_searched;
                            let fusions = std::mem::take(&mut played.fusions);
                            let winner = match played.ending.outcome {
                                Outcome::Won(w) if !capped => Some(w.index()),
                                _ => None,
                            };
                            for (mode, w) in &mut also {
                                let (rec, _) = record_searched(played.game.clone(), &played.dealt, &played.ending, *mode, table(), Some(&played.roots));
                                w.write(&rec).map_err(|e| e.to_string())?;
                            }
                            let (rec, failures) = record(played, options.mode);
                            let samples = rec.samples.len() as u64;
                            let visited = rec.samples.iter().filter(|s| s.visits.len() > 1).count() as u64;
                            writer.write(&rec).map_err(|e| e.to_string())?;
                            let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                            t.games += 1;
                            t.samples += samples;
                            t.visit_samples += visited;
                            t.capped += capped as u64;
                            t.answers += answers as u64;
                            t.searched += searched as u64;
                            t.simulations += sims;
                            t.evaluations += evals;
                            t.network_fallbacks += fallbacks as u64;
                            t.turns += turns as u64;
                            t.game_seconds += secs;
                            t.timing.add(&timing);
                            t.full_searched += full_searched as u64;
                            for shape in &shapes {
                                for (name, v) in shape_measures(shape) {
                                    *t.shape.entry(name).or_default().entry(v).or_default() += 1;
                                }
                            }
                            for f in &fusions {
                                t.fusion_roots += 1;
                                t.fusion_split += (f.distinct > 1) as u64;
                                t.fusion_worlds += f.worlds as u64;
                                t.fusion_plurality += f.plurality as u64;
                            }
                            for n in &pairing.names {
                                *t.seats.entry(n.clone()).or_default() += 1;
                            }
                            if let Some(w) = winner {
                                *t.wins.entry(pairing.names[w].clone()).or_default() += 1;
                                t.decided += 1;
                                t.first_wins += (w == first) as u64;
                            }
                            for (k, n) in failures {
                                *t.failures.entry(k).or_default() += n;
                            }
                            if t.games.is_multiple_of(if progress { 10 } else { 50 }) {
                                let s = started.elapsed().as_secs_f64();
                                let rate = t.games as f64 * 3600.0 / s.max(1e-9);
                                if progress {
                                    const WIDTH: usize = 28;
                                    let fraction = (t.games as f64 / run.games.max(1) as f64).min(1.0);
                                    let filled = (WIDTH as f64 * fraction) as usize;
                                    eprint!(
                                        "\rselfplay [{}{}] {}/{} ({:.1}%) {:.0} games/h, {:.1} searched/s, {} capped, {} dropped\x1b[K",
                                        "#".repeat(filled),
                                        "-".repeat(WIDTH - filled),
                                        t.games,
                                        run.games,
                                        fraction * 100.0,
                                        rate,
                                        t.searched as f64 / s.max(1e-9),
                                        t.capped,
                                        t.dropped
                                    );
                                    let _ = std::io::stderr().flush();
                                } else {
                                    eprintln!(
                                        "selfplay: {} games, {} samples in {:.0} s ({:.0} games/h, {:.1} searched decisions/s), {} capped, {} dropped",
                                        t.games,
                                        t.samples,
                                        s,
                                        rate,
                                        t.searched as f64 / s.max(1e-9),
                                        t.capped,
                                        t.dropped
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                            t.dropped += 1;
                            let key: String = e.chars().take(120).collect();
                            *t.errors.entry(key).or_default() += 1;
                        }
                    }
                }
                for (_, w) in &mut also {
                    w.close().map_err(|e| e.to_string())?;
                }
                writer.close().map_err(|e| e.to_string())
            });
            lanes.push(worker.map_err(|e| e.to_string())?);
        }
        let mut first_error = None;
        for lane in lanes {
            match lane.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => first_error = first_error.or(Some(e)),
                Err(_) => first_error = first_error.or(Some("a lane panicked".to_string())),
            }
        }
        first_error.map_or(Ok(()), Err)
    })?;
    let t = totals.into_inner().unwrap_or_else(|e| e.into_inner());
    if progress {
        eprintln!();
    }
    let seconds = started.elapsed().as_secs_f64();
    let this_games = t.games.saturating_sub(resumed_games) as f64;
    let this_samples = t.samples.saturating_sub(resumed_samples) as f64;
    let shape: serde_json::Map<String, Value> = t
        .shape
        .iter()
        .map(|(name, h)| {
            let mean = h.iter().map(|(v, n)| v * n).sum::<u64>() as f64 / h.values().sum::<u64>().max(1) as f64;
            let histogram: BTreeMap<String, u64> = h.iter().map(|(v, n)| (v.to_string(), *n)).collect();
            let summary = json!({
                "Median": quantile(h, 0.5),
                "P90": quantile(h, 0.9),
                "Max": h.keys().last(),
                "Mean": mean,
                "Histogram": histogram,
            });
            (name.to_string(), summary)
        })
        .collect();
    let fusion = json!({
        "Roots": t.fusion_roots,
        "SplitRate": if t.fusion_roots > 0 { t.fusion_split as f64 / t.fusion_roots as f64 } else { 0.0 },
        "PluralityShare": if t.fusion_worlds > 0 { t.fusion_plurality as f64 / t.fusion_worlds as f64 } else { 0.0 },
    });
    let timing = json!({
            "AnsweringSeconds": t.timing.answering,
            "SearchingSeconds": t.timing.searching,
            "EncodingSeconds": t.timing.encoding,
            "EvaluatingSeconds": t.timing.evaluating,
            "EngineAndTreeSeconds": t.timing.searching - t.timing.encoding - t.timing.evaluating,
            "OutsideSearchSeconds": t.timing.answering - t.timing.searching,
            "GameSeconds": t.game_seconds,
            "LinesSeconds": t.timing.play,
            "LinesSetupSeconds": t.timing.setup,
            "LinesGotoSeconds": t.timing.goto,
            "LinesObserveSeconds": t.timing.observe,
        });
    let budget = json!({
        "Batch": options.batch,
        "FullSimulations": options.full_simulations,
        "FullShare": options.full_share,
        "FullSearched": t.full_searched,
    });
    let report = json!({
        "Dir": run.out.display().to_string(),
        "Producer": run.producer,
        "Mode": mode_name(options.mode),
        "Simulations": options.simulations,
        "Sampled": options.sampled,
        "Temperature": options.temperature,
        "MaxTurns": options.max_turns,
        "MaxAnswers": options.max_answers,
        "Lanes": run.lanes,
        "Seeds": format!("{}..{}", run.seed, run.seed + run.games as u64),
        "Pairings": run.pairings.iter().map(|p| format!("{}/{}", p.names[0], p.names[1])).collect::<Vec<_>>(),
        "Games": t.games,
        "Resumed": resumed_games,
        "GamesThisRun": t.games.saturating_sub(resumed_games),
        "Samples": t.samples,
        "SamplesThisRun": t.samples.saturating_sub(resumed_samples),
        "VisitSamples": t.visit_samples,
        "VisitSamplesThisRun": t.visit_samples.saturating_sub(resumed_visits),
        "Capped": t.capped,
        "Dropped": t.dropped,
        "Errors": t.errors,
        "Answers": t.answers,
        "Searched": t.searched,
        "SimulationsRun": t.simulations,
        "Evaluations": t.evaluations,
        "NetworkFallbacks": t.network_fallbacks,
        "MeanTurns": if t.games > 0 { t.turns as f64 / t.games as f64 } else { 0.0 },
        "Seconds": seconds,
        "GameSecondsMean": if this_games > 0.0 { t.game_seconds / this_games } else { 0.0 },
        "GamesPerHour": this_games * 3600.0 / seconds.max(1e-9),
        "SamplesPerHour": this_samples * 3600.0 / seconds.max(1e-9),
        "SearchedPerSecond": t.searched as f64 / seconds.max(1e-9),
        "EvaluationsPerSecond": t.evaluations as f64 / seconds.max(1e-9),
        "Wins": t.wins,
        "SeatsByDeck": t.seats,
        "FirstPlayerWinRate": if t.decided > 0 { t.first_wins as f64 / t.decided as f64 } else { 0.0 },
        "Failures": t.failures,
        "Shape": shape,
        "Budget": budget,
        "Timing": timing,
        "Fusion": fusion,
    });
    let _ = std::fs::write(
        run.out.join("selfplay-report.json"),
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
    Ok(report)
}

/// A model directory's identity for a producer string: its name and a hash of its manifest and graphs.
pub fn model_id(dir: &Path) -> String {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().is_some_and(|n| {
                        n == "manifest.json" || n.to_string_lossy().ends_with(".onnx")
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for f in files {
        if let Ok(bytes) = std::fs::read(&f) {
            h.write(&bytes);
        }
    }
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string());
    format!("{name}:{:016x}", h.finish())
}
