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
//! Beside the champion against itself, a share of the games can be played against earlier champions
//! (`Options::opponents`, the learner in one seat) and a share can begin from scenario positions (`Options::starts`, a
//! `.cantrip` file's position once its last line is played) rather than a deal. Each game's opponent, learner seat and
//! start come from its index, so a resumed run deals them alike.
//!
//! A resumed run (`Run::resume`) reads the closed shards already in `out` and skips the games whose seeds they hold:
//! game `i` is always dealt from `seed + i`, and its pairing, seats and first player come from `i`, so the rest of the
//! schedule is unchanged. A `.part` shard that reads whole is recovered and renamed; one that ends inside a game has the
//! games before its cut written into a closed shard named after it (`<name>.recovered.shard.zst`), and only the cut
//! game is played again. What remains is decided by the games present, never by counting shards, so a shard a stopped
//! run closed partly full counts for the games it holds; and `Options::also`'s directories are read the same way, a
//! game they lack played again for them alone. A lock file (`selfplay.lock`) keeps a second process out of a live run.
//!
//! The lanes are a ceiling: `Run::govern` (`crate::governor`) decides how many play at once and when each next game
//! starts - memory admission, scaling to the first limiter, and a graceful stop that closes every shard - and never
//! how a game is played, so a game's record is the same whatever the lanes, the scheduling or the memory.

use std::collections::{BTreeMap, HashSet};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cantrip_ai::net::{self, Choice, Payment};
use cantrip_ai::record::{Dealt, Ending, Failures, Root, Roots, record_searched, table};
use cantrip_ai::search::{self, Config, Fusion, Shape, WallClock};
use cantrip_ai::shard::{
    GameRecord, Header, ShardReader, ShardWriter, mode_name, read as read_shard, shards as shard_files, write as write_shard,
};
use cantrip_ai::trace::{Seats, Trace};
use cantrip_ai::belief::{self, Belief};
use cantrip_ai::{Evaluator, Knowledge, Prior, TractEvaluator};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_script::{Advance, GameScript, Scripted};
use serde_json::{Value, json};

use crate::STACK_SIZE;
use crate::governor::{self, Govern, Governor, Meter};

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
    /// Leaves a full search sends at once; 0 is `batch`. A full search's budget is many times the fast one's, so a
    /// batch as large a share of it as `batch` is of the fast one fills the server's batches with far fewer round trips.
    pub full_batch: usize,
    /// A turn that takes this many answers makes the game a runaway: a loop, not play (no game gen1 finished took more
    /// than 53 in a turn). A runaway is ended at once and never recorded: its trace goes to `OUT/runaways.jsonl` with
    /// why, and a resumed run passes over its seed. 0, off.
    pub max_turn_answers: u32,
    /// A game played for longer than this many seconds is a runaway too. 0, off, the default: how long a game takes
    /// is the machine's load as much as the game's (at 64 lanes on one GPU a long, normal game passes 600 s), so the
    /// guards against a runaway count the game's own work instead (`max_turn_answers`, `max_answers`, and a decision's
    /// steps, `cantrip_ai::MAX_STEPS`).
    pub max_game_seconds: f64,
    /// Search settings by name (`search::Config::set`, `Key=Value`: `PriorTemperature=1.5`, `CPuct=2`, `Prune=false`,
    /// `Gumbel=true`, ...), for every searched decision; and for the full searches alone (playout-cap randomisation's
    /// `full_simulations`: `EarlyStop=256`, `Snapshots=256,1024`, ...), after them. None by default.
    pub search: Vec<String>,
    pub full_search: Vec<String>,
    /// Earlier champions a share of the games is played against, the learner (the model) in one seat: none by default,
    /// every game the model against itself. An opponent searches at the fast budget (`simulations`, no playout cap);
    /// its decisions are recorded one-hot with no root value, and the game's meta names the learner's seats
    /// (`GameMeta::learners`), so only the learner's decisions are search targets.
    pub opponents: Vec<Opponent>,
    /// Scenario positions a share `start_share` of the games begins from instead of a deal (`Starts`); none by default.
    pub starts: Option<Arc<Starts>>,
    pub start_share: f32,
    /// Worlds dealt by a belief head (`cantrip_ai::belief`): the learner's searches in Decklist mode deal an opponent's
    /// hidden hand by the belief head of the model in this directory (on the CPU, once a search), its other unseen
    /// cards uniformly. None, the default: every card dealt uniformly, as before. An earlier champion's searches deal
    /// uniformly.
    pub belief: Option<PathBuf>,
}

/// An earlier champion played against (`Options::opponents`): its name, as the seats and the report say it, and the
/// share of the games it plays.
#[derive(Clone, Debug)]
pub struct Opponent {
    pub name: String,
    pub share: f32,
}

/// `DIR` or `DIR:SHARE` (a share in [0, 1]; a Windows drive's colon is no share): the path and its share, `default`
/// where none is given.
pub fn path_share(spec: &str, default: f32) -> Result<(PathBuf, f32), String> {
    match spec.rsplit_once(':') {
        Some((dir, share)) if !dir.is_empty() && share.parse::<f32>().is_ok() => {
            let share: f32 = share.parse().unwrap_or(default);
            if !(0.0..=1.0).contains(&share) {
                return Err(format!("{spec}: a share is between 0 and 1"));
            }
            Ok((PathBuf::from(dir), share))
        }
        _ => Ok((PathBuf::from(spec), default)),
    }
}

/// A scenario's position a game begins from (`Options::starts`): the game once the `.cantrip` file's last line is
/// played (the runner answering every seat as the script means, and stopping at the first priority with an empty stack
/// after it: `cantrip_script::Advance::Done`), with its decklists.
///
/// A scenario's libraries are in its decklists' order, so every game from it would draw the same cards: each game is
/// dealt from it anew by its seed (`Start::game_for`), each library's cards no player has seen put in another order
/// among their own places, and the game's generator reseeded. A card any seat knows (`Knowledge`: one it saw, or saw put
/// there) keeps its place, so nothing a seat knows changes; and a script can keep its libraries' tops as written with a
/// comment line `# selfplay: keep-top N` (the top N cards of every library) or keep them whole with
/// `# selfplay: keep-library`.
pub struct Start {
    pub name: String,
    /// Behind a lock only to be shared by the lanes (a `Game` is not `Sync`): each game is dealt from a copy.
    pub game: Mutex<Game>,
    pub lists: [Vec<String>; 2],
    /// The top cards of every library kept in the scenario's order (`# selfplay: keep-top N`; all of them for
    /// `# selfplay: keep-library`).
    pub keep_top: usize,
}

/// A scenario's `# selfplay: ...` lines (`Start`): the library tops it keeps in its own order.
fn keep_top(text: &str) -> Result<usize, String> {
    let mut keep = 0usize;
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix('#').map(str::trim).and_then(|l| l.strip_prefix("selfplay:")) else { continue };
        let words: Vec<&str> = rest.split_whitespace().collect();
        match words.as_slice() {
            ["keep-library"] => keep = usize::MAX,
            ["keep-top", n] => keep = keep.max(n.parse().map_err(|_| format!("`{}`: keep-top takes a count", line.trim()))?),
            _ => return Err(format!("`{}`: a selfplay line is `keep-top N` or `keep-library`", line.trim())),
        }
    }
    Ok(keep)
}

/// The scenario positions of a directory (`Start`).
pub struct Starts {
    pub dir: PathBuf,
    pub list: Vec<Start>,
}

impl std::fmt::Debug for Starts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.list.iter().map(|s| s.name.as_str()).collect();
        f.debug_struct("Starts").field("dir", &self.dir).field("list", &names).finish()
    }
}

impl Start {
    /// The position of the scenario in `path` (see `Start`): two seats, the game not over, a seat asked.
    pub fn load(path: &Path) -> Result<Start, String> {
        let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let keep_top = keep_top(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let script = GameScript::load(path).map_err(|e| format!("{}: {}", path.display(), e.0))?;
        if script.decks.len() != 2 {
            return Err(format!("{}: self-play starts two seats, not {}", path.display(), script.decks.len()));
        }
        let lists = [script.decklist(0), script.decklist(1)];
        let mut run = Scripted::deal(script).map_err(|e| format!("{}: {}", path.display(), e.message))?;
        loop {
            match run.advance(&[]).map_err(|e| format!("{}: {}", path.display(), e.message))? {
                Advance::Done => break,
                Advance::Check(_) => {}
                Advance::Ask(_) => return Err(format!("{}: the runner stopped on a question no seat drives", path.display())),
            }
        }
        let game = std::mem::replace(run.game(), deal(&[vec!["Island".into()], vec!["Island".into()]], 0, false, Some(PlayerId(0)))?);
        if game.is_over() || game.pending().is_none() {
            return Err(format!("{}: the game is over at its position", path.display()));
        }
        Ok(Start { name, game: Mutex::new(game), lists, keep_top })
    }

    /// The game of seed `seed` from this position (see `Start`): a copy going on from it (`Game::fork_for_search`: its
    /// own tree, rooted at the position), its generator reseeded from `seed`, and each library's cards that no seat
    /// knows, below the top `keep_top`, put in an order drawn from `seed` among their own places
    /// (`GameState::reorder_library`); the question waiting is asked again of the dealt cards where any moved
    /// (`Game::ask_again`). The same seed deals the same game: self-play, reanalyse and the audit stand a traced game in
    /// again with it.
    pub fn game_for(&self, seed: u64) -> Game {
        let base = self.game.lock().unwrap_or_else(|e| e.into_inner());
        let mut game = base.fork_for_search();
        drop(base);
        if self.keep_top == usize::MAX {
            return game;
        }
        game.reseed(splitmix(seed ^ 0x57a7_de41));
        let knowledge: Vec<Knowledge> = game.state.seats().map(|p| Knowledge::of(&game, p)).collect();
        let known = |id: ObjectId| knowledge.iter().any(|k| k.knows(id).is_some());
        let mut rng = Rng::new(splitmix(seed ^ 0x11b5_a7f1));
        let players = game.state.players.len();
        let mut moved = false;
        for p in game.state.seats().collect::<Vec<_>>() {
            let slot = Zone::library(p).slot(players);
            let zone = game.state.zones[slot].clone();
            // What a resolution in progress put there keeps its place (`zone_order_fixed`), as a world's deal keeps it.
            let fixed = game.state.zone_order_fixed(slot);
            let places: Vec<usize> =
                (self.keep_top.min(zone.len())..zone.len()).filter(|&i| !known(zone[i]) && !fixed.contains(&zone[i])).collect();
            if places.len() < 2 {
                continue;
            }
            let mut cards: Vec<ObjectId> = places.iter().map(|&i| zone[i]).collect();
            for i in (1..cards.len()).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                cards.swap(i, j);
            }
            let mut order = zone.clone();
            for (&i, id) in places.iter().zip(cards) {
                order[i] = id;
            }
            moved |= order != zone;
            game.state.reorder_library(slot, order);
        }
        if moved {
            game.ask_again();
        }
        game
    }
}

impl Starts {
    /// Every `.cantrip` file in `dir` (sorted), each a `Start`.
    pub fn load(dir: &Path) -> Result<Starts, String> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "cantrip"))
            .collect();
        paths.sort();
        if paths.is_empty() {
            return Err(format!("{}: no .cantrip scenario", dir.display()));
        }
        let list = paths.iter().map(|p| Start::load(p)).collect::<Result<Vec<_>, _>>()?;
        Ok(Starts { dir: dir.to_path_buf(), list })
    }
}

/// Who plays a game's two seats: an evaluator each, whether it is the learner's (`Options::opponents`), and the name
/// its seat is recorded under.
pub struct Players<'a> {
    pub evaluators: [&'a dyn Evaluator; 2],
    pub learner: [bool; 2],
    pub names: [String; 2],
    /// The belief head each seat's worlds are dealt by (`Options::belief`), none: uniformly.
    pub beliefs: [Option<&'a dyn Belief>; 2],
    /// Set, the game stops at its next decision with `governor::STOPPED` (the governor's graceful stop).
    pub abort: Option<&'a AtomicBool>,
    /// Counts the searches' work as the game goes (the governor's throughput).
    pub meter: Option<&'a Meter>,
}

impl<'a> Players<'a> {
    /// The model against itself, `model` naming both seats.
    pub fn mirror(evaluator: &'a dyn Evaluator, model: &str) -> Players<'a> {
        Players { evaluators: [evaluator, evaluator], learner: [true, true], names: [model.to_string(), model.to_string()], beliefs: [None, None], abort: None, meter: None }
    }
}

/// A unit draw from a hash.
fn unit(x: u64) -> f64 {
    (splitmix(x) >> 11) as f64 / (1u64 << 53) as f64
}

/// Game `i`'s opponent (by its place in `Options::opponents`) and the learner's seat, and its start (by its place in
/// `Options::starts`): each a draw of its own from the run's seed and the index, so the defaults leave every game as it
/// was.
pub fn assignment(i: usize, seed: u64, options: &Options) -> (Option<(usize, usize)>, Option<usize>) {
    let mut opponent = None;
    if !options.opponents.is_empty() {
        let u = unit(seed ^ ((i as u64) << 20) ^ 0x0990_7e47);
        let mut at = 0.0f64;
        for (j, o) in options.opponents.iter().enumerate() {
            at += f64::from(o.share);
            if u < at {
                let learner_seat = (splitmix(seed ^ ((i as u64) << 22) ^ 0x1ea7) >> 9) as usize & 1;
                opponent = Some((j, learner_seat));
                break;
            }
        }
    }
    let start = options.starts.as_ref().filter(|s| !s.list.is_empty()).and_then(|starts| {
        (unit(seed ^ ((i as u64) << 18) ^ 0x57a7_7e) < f64::from(options.start_share))
            .then(|| (splitmix(seed ^ ((i as u64) << 26) ^ 0x57a7) % starts.list.len() as u64) as usize)
    });
    (opponent, start)
}

impl Options {
    fn runaway_checked(&self) -> bool {
        self.max_turn_answers > 0 || self.max_game_seconds > 0.0
    }
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
            full_batch: 0,
            max_turn_answers: 300,
            max_game_seconds: 0.0,
            search: Vec::new(),
            full_search: Vec::new(),
            opponents: Vec::new(),
            starts: None,
            start_share: 0.0,
            belief: None,
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
    /// Why the game was ended as a runaway (`Options::max_turn_answers`, `max_game_seconds`), if it was.
    pub runaway: Option<String>,
    /// For each of the learner's searches that kept budget snapshots (`snapshot_agreement`, with `Snapshots=...` in
    /// `Options::search` or `full_search`): each earlier snapshot's count asked and whether its most visited root option
    /// is the finished search's.
    pub snapshots: Vec<(usize, bool)>,
    /// Searches whose belief head failed (`Options::belief`), dealt uniformly instead.
    pub belief_errors: u32,
}

/// A search's budget snapshots (`search::Config::snapshots`, `Searched::snapshots`) agreeing with its end: each
/// snapshot taken before the end, by the count of simulations it was asked at (the counts sorted, as the search takes
/// them; a count the search stopped before has none), and whether its most visited root option (a whole answer's
/// tokens, or the step's option) is the finished search's (the last snapshot's, taken at the end). Empty where the
/// search kept none.
fn snapshot_agreement(s: &search::Searched, asked: &[usize]) -> Vec<(usize, bool)> {
    let Some((last, earlier)) = s.snapshots.split_last() else {
        return Vec::new();
    };
    let Some(best) = &last.best else {
        return Vec::new();
    };
    let mut asked = asked.to_vec();
    asked.sort_unstable();
    asked.into_iter().zip(earlier).map(|(at, snap)| (at, snap.best.as_ref() == Some(best))).collect()
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
    play_with(&Players::mirror(evaluator, &options.model), pairing, seed, first, None, options, prior)
}

/// `play`, with `players` in the seats, and from `start`'s position instead of a deal where one is given (the
/// scenario's decks then, not `pairing`'s, and nobody chooses who goes first).
#[allow(clippy::too_many_arguments)]
pub fn play_with(
    players: &Players,
    pairing: &Pairing,
    seed: u64,
    first: usize,
    start: Option<&Start>,
    options: &Options,
    prior: &Prior,
) -> Result<Played, String> {
    let started = Instant::now();
    let decks = match start {
        Some(s) => s.lists.to_vec(),
        None => vec![pairing.lists[0].clone(), pairing.lists[1].clone()],
    };
    let mut game = match start {
        Some(s) => s.game_for(seed),
        None => deal(&decks, seed, true, None)?,
    };
    let start_node = game.at;
    let mut snapshot_agreement: Vec<(usize, bool)> = Vec::new();
    let mut belief_errors = 0u32;
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
    let mut runaway: Option<String> = None;
    let (mut this_turn, mut turn_answers) = (0u32, 0u32);
    // Kept for a runaway's diagnosis whenever runaways are checked, written only for one (or with `traces`).
    let mut trace = (options.traces || options.runaway_checked()).then(|| {
        let mut t = Trace::new(seed, pairing.names.to_vec(), decks.clone());
        t.start = start.map(|s| s.name.clone());
        (t, Seats::new(&game))
    });
    while !game.is_over() {
        if players.abort.is_some_and(|a| a.load(Ordering::Relaxed)) {
            return Err(governor::STOPPED.to_string());
        }
        let Some(request) = game.pending().cloned() else {
            break;
        };
        if game.state.turn.number > options.max_turns || answers >= options.max_answers {
            capped = true;
            break;
        }
        if game.state.turn.number != this_turn {
            (this_turn, turn_answers) = (game.state.turn.number, 0);
        }
        if options.max_turn_answers > 0 && turn_answers >= options.max_turn_answers {
            runaway = Some(format!("{turn_answers} answers in turn {this_turn}"));
        } else if options.max_game_seconds > 0.0 && started.elapsed().as_secs_f64() > options.max_game_seconds {
            runaway = Some(format!("over {:.0} s, at turn {this_turn}", options.max_game_seconds));
        }
        if runaway.is_some() {
            capped = true;
            break;
        }
        turn_answers += 1;
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
        let evaluator = players.evaluators[p];
        let learner = players.learner[p];
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
        // Playout-cap randomisation: this decision's budget, full or fast, by a draw of its own. An earlier champion
        // (`Options::opponents`) is always at the fast budget, as nothing of its search is a target.
        let capped_off = options.full_simulations == 0;
        let full = learner
            && (capped_off
                || (splitmix(seed ^ ((request.id as u64) << 28) ^ 0xf011) >> 11) as f64 / (1u64 << 53) as f64 <= options.full_share as f64);
        let budget = if !capped_off && full { options.full_simulations } else { options.simulations };
        let mut config = Config {
            choice,
            seed: splitmix(seed ^ ((request.id as u64) << 24) ^ 0x5ea2c4),
            worlds: options.worlds,
            batch: if !capped_off && full && options.full_batch > 0 { options.full_batch } else { options.batch.max(1) },
            ..Config::simulations(budget)
        };
        let full_settings = if !capped_off && full { &options.full_search[..] } else { &[][..] };
        for pair in options.search.iter().chain(full_settings) {
            config.set_pair(pair)?;
        }
        let k = &knowledge[p];
        let tree = &mut trees[p];
        let asked = Instant::now();
        // The opponents' hands dealt by the seat's belief head, where it has one (`Options::belief`).
        let weighted = match players.beliefs[p] {
            Some(b) => match belief::hand_weights(&game, &request, k, options.mode, b, table()) {
                Ok(Some(w)) => Some(prior.with_hand(w)),
                Ok(None) => None,
                Err(_) => {
                    belief_errors += 1;
                    None
                }
            },
            None => None,
        };
        let prior = weighted.as_ref().unwrap_or(prior);
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
            Err(e) if e.0.starts_with(cantrip_ai::RUNAWAY) => {
                runaway = Some(format!("{} at turn {}", e.0, game.state.turn.number));
                capped = true;
                break;
            }
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
            if let Some(m) = players.meter {
                m.evaluations.fetch_add(s.evaluations as u64, Ordering::Relaxed);
                m.searched.fetch_add(1, Ordering::Relaxed);
            }
            if !s.visits.is_empty() && !learner {
                searched_by[p] += 1;
            } else if !s.visits.is_empty() {
                snapshot_agreement.extend(self::snapshot_agreement(s, &config.snapshots));
                searched += 1;
                searched_by[p] += 1;
                full_searched += u32::from(full && !capped_off);
                // The tree's shape of the searches that are policy targets.
                if full {
                    shapes.push(s.shape);
                }
                fusions.extend(s.fusion);
                roots.insert(at, Root::of(s, full));
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
            .map(|i| format!("{}{i}", players.names[i - 1]))
            .collect(),
        decks: pairing.names.to_vec(),
        auto_start: start.is_none(),
        learners: if players.learner.iter().all(|&l| l) {
            Vec::new()
        } else {
            (0..seats).filter(|&q| players.learner[q]).map(|q| q as u8 + 1).collect()
        },
        start: start.map(|s| (start_node, s.name.clone())),
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
        runaway,
        snapshots: snapshot_agreement,
        belief_errors,
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
    /// How many games run at once and when each starts (`crate::governor`); `Govern::default()`, off: every lane
    /// plays, no limit.
    pub govern: Govern,
    /// A grant file (`Work`, `parse_grant`): only the games it grants are played, as it grants them, until it says
    /// `end`. None: every game of `games`.
    pub grant: Option<PathBuf>,
    /// The machine playing, as the run's report and status name it (the distributed driver's record of which host
    /// played which game: a game replays exactly only on the hardware that played it).
    pub host: Option<String>,
}

/// The stop file in a run's directory (`Govern::stop_file`): there, the run stops as at Ctrl-C; saying `now`, its
/// games in flight are stopped too.
pub const STOP_FILE: &str = "selfplay.stop";

/// A run's status, rewritten every few seconds while it plays and once at its end (`run_with`): what the distributed
/// driver reads of a worker's progress.
pub const STATUS: &str = "selfplay-status.json";

/// How often the status file is written.
const STATUS_EVERY: Duration = Duration::from_secs(5);

/// Game `i`'s pairing, seats and first player: the pairings in turn, each dealt with the decks either way round and
/// either seat first.
pub fn schedule(i: usize, pairings: usize) -> (usize, bool, usize) {
    let p = i % pairings;
    let swap = (i / pairings) % 2 == 1;
    let first = (i / (2 * pairings)) % 2;
    (p, swap, first)
}

/// A grant file's games (`Run::grant`): the indices it grants, in its order, and whether it is closed (`end`). One
/// line each: `A..B` (the games A to B - 1), `A` (one game), `end` (no more will be granted), `#` a comment. Only
/// lines that end in a newline count, so a file read as it is being written grants nothing it does not mean to (the
/// distributed driver writes it whole and renames it, besides). A game at or past `games` is an error.
pub fn parse_grant(text: &str, games: usize) -> Result<(Vec<std::ops::Range<usize>>, bool), String> {
    let mut ranges = Vec::new();
    let mut ended = false;
    for line in text.split_inclusive('\n').filter(|l| l.ends_with('\n')).map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "end" {
            ended = true;
            continue;
        }
        let number = |t: &str| t.trim().parse::<usize>().map_err(|_| format!("a grant line is A..B, A or end, not {line}"));
        let range = match line.split_once("..") {
            Some((a, b)) => number(a)?..number(b)?,
            None => {
                let a = number(line)?;
                a..a + 1
            }
        };
        if range.end > games {
            return Err(format!("the grant {line} passes the run's {games} games"));
        }
        ranges.push(range);
    }
    Ok((ranges, ended))
}

/// What a lane plays next (`Work::take`).
#[derive(Debug, PartialEq, Eq)]
pub enum Pick {
    Game(usize),
    /// Nothing granted is left yet, and more may be: ask again in a moment.
    Wait,
    /// No game is left to play.
    Done,
}

/// The games a run plays, handed to the lanes one at a time: every index from 0 in turn, or with a grant file
/// (`Run::grant`) the indices it grants, in its order, read again as it grows until it says `end` (the distributed
/// driver's work split: docs/ai-training.md, "Distributed self-play"). A grant never takes back a game; an index is
/// handed out once, and one the run's shards already hold (`resumed`) is passed over.
pub struct Work {
    games: usize,
    grant: Option<PathBuf>,
    state: Mutex<WorkState>,
}

struct WorkState {
    queue: std::collections::VecDeque<usize>,
    known: Vec<bool>,
    ended: bool,
    read_at: Option<Instant>,
    granted: usize,
    taken: usize,
    error: Option<String>,
}

/// How often a lane with nothing granted reads the grant file again.
const GRANT_POLL: Duration = Duration::from_millis(500);

impl Work {
    pub fn new(games: usize, grant: Option<PathBuf>) -> Work {
        let state = WorkState {
            queue: if grant.is_none() { (0..games).collect() } else { Default::default() },
            known: vec![grant.is_none(); if grant.is_none() { 0 } else { games }],
            ended: grant.is_none(),
            read_at: None,
            granted: if grant.is_none() { games } else { 0 },
            taken: 0,
            error: None,
        };
        Work { games, grant, state: Mutex::new(state) }
    }

    /// The next game to play, `done` passing over those already played.
    pub fn take(&self, done: impl Fn(usize) -> bool) -> Result<Pick, String> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            while let Some(i) = s.queue.pop_front() {
                if !done(i) {
                    s.taken += 1;
                    return Ok(Pick::Game(i));
                }
            }
            if s.ended {
                return Ok(Pick::Done);
            }
            let Some(path) = &self.grant else { return Ok(Pick::Done) };
            if s.read_at.is_some_and(|t| t.elapsed() < GRANT_POLL) {
                return Ok(Pick::Wait);
            }
            s.read_at = Some(Instant::now());
            // Not there yet, or being replaced this moment: nothing new.
            let Ok(text) = std::fs::read_to_string(path) else { return Ok(Pick::Wait) };
            let (ranges, ended) = parse_grant(&text, self.games).map_err(|e| {
                let e = format!("{}: {e}", path.display());
                s.error = Some(e.clone());
                e
            })?;
            let before = s.queue.len();
            for i in ranges.into_iter().flatten() {
                if !s.known[i] {
                    s.known[i] = true;
                    s.granted += 1;
                    s.queue.push_back(i);
                }
            }
            s.ended = ended;
            if s.queue.len() == before && !ended {
                return Ok(Pick::Wait);
            }
        }
    }

    /// The games granted so far, those handed to a lane, and whether the grant is closed (for the status file).
    pub fn counts(&self) -> (usize, usize, bool) {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        (s.granted, s.taken, s.ended)
    }
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
    /// Games ended as runaways (`Options::max_turn_answers`, `max_game_seconds`): not recorded, not in `games`.
    runaways: u64,
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
    /// This run's games against each earlier champion (`Options::opponents`): played, the learner's wins, and those
    /// decided.
    opponents: BTreeMap<String, [u64; 3]>,
    /// This run's games begun from each scenario position (`Options::starts`).
    starts: BTreeMap<String, u64>,
    /// Each budget snapshot's searches (`snapshots`), and those whose most visited root option was the finished
    /// search's.
    snapshots: BTreeMap<usize, [u64; 2]>,
    belief_errors: u64,
    /// Games the governor stopped at a decision (`governor::STOPPED`): not recorded, played again on a resume.
    stopped: u64,
}

/// Ends a governor and wakes its monitor (`run_with`): every lane waiting is let go, and the monitor returns.
struct FinishOnDrop<'a>(&'a Governor, std::thread::Thread);

impl Drop for FinishOnDrop<'_> {
    fn drop(&mut self) {
        self.0.finish();
        self.1.unpark();
    }
}

/// The lock file of a run's directory (`RunLock`).
pub const LOCK: &str = "selfplay.lock";

/// A run's hold on its directory: `selfplay.lock`, holding the process id, made anew or taken over from a process that
/// is gone; removed when the run ends. A second process finds it held and refuses, so it never recovers a live run's
/// `.part`s.
struct RunLock(PathBuf);

impl RunLock {
    fn take(dir: &Path) -> Result<RunLock, String> {
        let path = dir.join(LOCK);
        for _ in 0..3 {
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    write!(f, "{}", std::process::id()).and_then(|_| f.sync_all()).map_err(|e| format!("{}: {e}", path.display()))?;
                    return Ok(RunLock(path));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let pid = std::fs::read_to_string(&path).ok().and_then(|t| t.trim().parse::<u32>().ok());
                    let held = match pid {
                        Some(pid) => governor::process_alive(pid),
                        // Being written this moment, or left empty long ago.
                        None => std::fs::metadata(&path)
                            .and_then(|m| m.modified())
                            .is_ok_and(|t| t.elapsed().is_ok_and(|age| age < Duration::from_secs(10))),
                    };
                    if held {
                        return Err(format!(
                            "{} is in use by another self-play process{} ({}); wait for it, or remove the lock if it is gone",
                            dir.display(),
                            pid.map(|p| format!(" (pid {p})")).unwrap_or_default(),
                            path.display()
                        ));
                    }
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => return Err(format!("{}: {e}", path.display())),
            }
        }
        Err(format!("{}: the lock could not be taken", path.display()))
    }
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Ends the status thread, however the lanes end (`run_with`).
struct StopOnDrop<'a>(&'a AtomicBool, std::thread::Thread);

impl Drop for StopOnDrop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        self.1.unpark();
    }
}

/// `OUT/selfplay-status.json` (`STATUS`), written whole and renamed into place: the process, the games finished (all,
/// and this run's), the games granted and handed out, the rate, the governor's lanes and limiter, why it stopped, and
/// whether the run is over. A write that fails (a reader holding the file this moment) is skipped; the next one comes.
fn write_status(run: &Run, started: &Instant, resumed: u64, totals: &Mutex<Totals>, work: &Work, governor: &Governor, finished: bool) {
    let (games, runaways, dropped) = {
        let t = totals.lock().unwrap_or_else(|e| e.into_inner());
        (t.games, t.runaways, t.dropped)
    };
    let (granted, taken, ended) = work.counts();
    let seconds = started.elapsed().as_secs_f64();
    let this = games.saturating_sub(resumed);
    let status = json!({
        "Pid": std::process::id(),
        "Host": run.host,
        "Updated": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64()),
        "Seconds": seconds,
        "Games": games,
        "GamesThisRun": this,
        "Runaways": runaways,
        "Dropped": dropped,
        "GamesPerHour": this as f64 * 3600.0 / seconds.max(1e-9),
        "Granted": granted,
        "Taken": taken,
        "GrantEnded": ended,
        "Governor": governor.status(),
        "Finished": finished,
    });
    let path = run.out.join(STATUS);
    let temporary = run.out.join(format!("{STATUS}.tmp"));
    // The last one is the one a reader acts on: tried again while a reader holds the file (Windows refuses the rename).
    for _ in 0..if finished { 40 } else { 1 } {
        if std::fs::write(&temporary, serde_json::to_string_pretty(&status).unwrap_or_default()).is_ok()
            && std::fs::rename(&temporary, &path).is_ok()
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The header a run's shards carry (`Header::new`), as JSON: what the distributed driver holds every shard to, asked of
/// this binary and this model before any game (`cantrip-selfplay --print-header`), never learnt from a shard.
pub fn expected_header(run: &Run, options: &Options) -> Value {
    let h = Header::new(table(), &run.producer, &format!("{}..{}", run.seed, run.seed + run.games as u64), options.mode);
    json!({ "producer": h.producer, "mode": h.mode, "text_hash": h.text_hash, "seeds": h.seeds, "encoder_hash": h.encoder_hash, "schema": h.schema, "format": h.format })
}

/// Ends a governor's admissions when a lane ends, however it ends (`run_with`).
struct CloseOnExit<'a>(&'a Governor);

impl Drop for CloseOnExit<'_> {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// The runaways' file in a run's directory: a JSON line each (`Options::max_turn_answers`).
pub const RUNAWAYS: &str = "runaways.jsonl";

/// A `.part` shard an interrupted writer left, recovered as far as it reads (see the module), once its header is
/// `expect`ed: whole, it is renamed to its final name; cut inside a game, the games before the cut are written into a
/// closed shard named after it (`<name>.recovered.shard.zst`, by way of a `.recovered.tmp` renamed over any earlier
/// one, so a recovery interrupted at any step and done again leaves each game once), and then the `.part` is removed.
/// One with no header is removed. Returns its header and games.
fn recover_part(path: &Path, expect: impl Fn(&Header) -> Result<(), String>) -> Result<Option<(Header, Vec<GameRecord>)>, String> {
    let at = |e: std::io::Error| format!("{}: {e}", path.display());
    let Ok(mut reader) = ShardReader::open(path) else {
        std::fs::remove_file(path).map_err(at)?;
        return Ok(None);
    };
    expect(&reader.header)?;
    let mut games = Vec::new();
    let whole = loop {
        match reader.game() {
            Ok(Some(g)) => games.push(g),
            Ok(None) => break true,
            Err(_) => break false,
        }
    };
    let header = reader.header.clone();
    drop(reader);
    if whole {
        std::fs::rename(path, path.with_extension("")).map_err(at)?;
    } else {
        if !games.is_empty() {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let stem = name.trim_end_matches(".part").trim_end_matches(&format!(".{}", cantrip_ai::shard::EXTENSION));
            let recovered = path.with_file_name(format!("{stem}.{RECOVERED}.{}", cantrip_ai::shard::EXTENSION));
            let temporary = path.with_file_name(format!("{stem}.{RECOVERED}.tmp"));
            write_shard(&temporary, &header, &games).map_err(at)?;
            std::fs::rename(&temporary, &recovered).map_err(at)?;
        }
        std::fs::remove_file(path).map_err(at)?;
    }
    Ok(Some((header, games)))
}

/// What a recovered shard's name says (`recover_part`).
const RECOVERED: &str = "recovered";

/// The leftovers of a recovery interrupted before its rename (`recover_part`), removed.
fn remove_recovery_leftovers(dir: &Path) -> Result<(), String> {
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok) {
        if e.file_name().to_string_lossy().ends_with(&format!(".{RECOVERED}.tmp")) {
            std::fs::remove_file(e.path()).map_err(|x| format!("{}: {x}", e.path().display()))?;
        }
    }
    Ok(())
}

/// Whether a shard's header is this run's, in `mode`.
fn check_header(path: &Path, header: &Header, run: &Run, mode: InfoMode) -> Result<(), String> {
    let seeds = format!("{}..{}", run.seed, run.seed + run.games as u64);
    if header.producer != run.producer || header.mode != mode_name(mode) || header.text_hash != table().hash || header.seeds != seeds {
        return Err(format!(
            "{} is from another self-play run (producer {}, mode {}, seeds {}); remove {} or resume the run it belongs to",
            path.display(),
            header.producer,
            header.mode,
            header.seeds,
            run.out.display()
        ));
    }
    Ok(())
}

/// What a run's directory already holds (`resumed`): the games its own shards hold, by index, those each of
/// `Options::also`'s directories holds, in its order, and the totals of the run's own.
struct Resumed {
    main: HashSet<usize>,
    also: Vec<HashSet<usize>>,
    totals: Totals,
}

impl Resumed {
    fn none(options: &Options) -> Resumed {
        Resumed { main: HashSet::new(), also: vec![HashSet::new(); options.also.len()], totals: Totals::default() }
    }

    /// Whether game `i` is in every directory it is written to (or a runaway): not played again.
    fn done(&self, i: usize) -> bool {
        self.main.contains(&i) && self.also.iter().all(|a| a.contains(&i))
    }
}

/// The game index of `game`, a game of `path`.
fn index_of(path: &Path, game: &GameRecord, run: &Run) -> Result<usize, String> {
    game.meta
        .seed
        .checked_sub(run.seed)
        .filter(|&i| i < run.games as u64)
        .map(|i| i as usize)
        .ok_or_else(|| format!("{}: game seed {} is not in {}..{}", path.display(), game.meta.seed, run.seed, run.seed + run.games as u64))
}

/// The games of the shards in `dir`, `.part`s recovered first (`recover_part`), each header checked for `mode`.
fn shards_in(dir: &Path, run: &Run, mode: InfoMode) -> Result<Vec<(PathBuf, Vec<GameRecord>)>, String> {
    let mut all = Vec::new();
    if !dir.is_dir() {
        return Ok(all);
    }
    remove_recovery_leftovers(dir)?;
    let is_part = |path: &Path| path.file_name().is_some_and(|n| n.to_string_lossy().ends_with(".part"));
    for path in shard_paths(dir)? {
        let games = if is_part(&path) {
            match recover_part(&path, |h| check_header(&path, h, run, mode))? {
                Some((_, games)) => games,
                None => continue,
            }
        } else {
            let (header, games) = read_shard(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            check_header(&path, &header, run, mode)?;
            games
        };
        all.push((path, games));
    }
    Ok(all)
}

/// What the shards in `out` already hold (`Resumed`): the game indices finished, and the totals those games make. Every
/// game a shard holds counts, however full the shard: a finished `.shard.zst` is read whole, and a `.part` as far as it
/// reads (`recover_part`). `Options::also`'s directories are read the same way: a game the run's own shards hold that
/// one of them lacks (a hard kill between the writes) is played again for it alone.
fn resumed(out: &Path, run: &Run, options: &Options) -> Result<Resumed, String> {
    let mut r = Resumed::none(options);
    if !out.exists() {
        return Ok(r);
    }
    let (done, t) = (&mut r.main, &mut r.totals);
    for (path, games) in shards_in(out, run, options.mode)? {
        for game in games {
            let index = index_of(&path, &game, run)?;
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
    for (k, &mode) in options.also.iter().enumerate() {
        for (path, games) in shards_in(&out.join(mode_name(mode)), run, mode)? {
            for game in games {
                r.also[k].insert(index_of(&path, &game, run)?);
            }
        }
    }
    let (done, t) = (&mut r.main, &mut r.totals);
    // The runaways: ended unrecorded, and not played again.
    let path = out.join(RUNAWAYS);
    if let Ok(text) = std::fs::read_to_string(&path) {
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let v: Value = serde_json::from_str(line).map_err(|e| format!("{}: {e}", path.display()))?;
            if let Some(i) = v["index"].as_u64().filter(|&i| i < run.games as u64)
                && done.insert(i as usize)
            {
                t.runaways += 1;
                for a in &mut r.also {
                    a.insert(i as usize);
                }
            }
        }
    }
    Ok(r)
}

/// Recovers the `.part`s a run killed outright left in `run.out` (`recover_part`) and plays nothing: what a resume does
/// first, done alone, so the games they hold are in closed shards before another process is given the rest (the
/// distributed driver, after a worker's self-play crashed). Takes the run's lock, so never a live run's. Returns the
/// games and runaways the directory then holds.
pub fn recover(run: &Run, options: &Options) -> Result<Value, String> {
    std::fs::create_dir_all(&run.out).map_err(|e| format!("{}: {e}", run.out.display()))?;
    let _lock = RunLock::take(&run.out)?;
    let r = resumed(&run.out, run, options)?;
    Ok(json!({ "Dir": run.out.display().to_string(), "Games": r.totals.games, "Runaways": r.totals.runaways }))
}

/// Plays `run.games` games on `run.lanes` lanes, each with its own evaluator from `load`, and writes their shards and a
/// report (`selfplay-report.json`) into `run.out`. Returns the report.
pub fn run<F>(run: &Run, options: &Options, prior: &Prior, load: F) -> Result<Value, String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    run_with(run, options, prior, load, |_| Err("no earlier champion can be loaded here".to_string()))
}

/// `run`, each lane loading each earlier champion of `Options::opponents` with `load_opponent` (by its place).
pub fn run_with<F, G>(run: &Run, options: &Options, prior: &Prior, load: F, load_opponent: G) -> Result<Value, String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
    G: Fn(usize) -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    let shares: f32 = options.opponents.iter().map(|o| o.share).sum();
    if shares > 1.0 + 1e-6 {
        return Err(format!("the opponents' shares sum to {shares}, more than every game"));
    }
    std::fs::create_dir_all(&run.out).map_err(|e| format!("{}: {e}", run.out.display()))?;
    let _lock = RunLock::take(&run.out)?;
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
    // Search settings, opponents, the belief deal and starts join the identity only where given, so a run made before
    // them resumes as it was.
    let mut identity = identity;
    if !options.search.is_empty() || !options.full_search.is_empty() {
        identity["search"] = json!(options.search);
        identity["full_search"] = json!(options.full_search);
    }
    if !options.opponents.is_empty() {
        identity["opponents"] = json!(options.opponents.iter().map(|o| format!("{}:{}", o.name, o.share)).collect::<Vec<_>>());
    }
    if let Some(dir) = &options.belief {
        identity["belief"] = json!(dir.display().to_string());
    }
    if let Some(starts) = options.starts.as_ref().filter(|_| options.start_share > 0.0) {
        identity["starts"] = json!({
            "share": options.start_share,
            "names": starts.list.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
        });
    }
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
    let mut completed = if run.resume { resumed(&run.out, run, options)? } else { Resumed::none(options) };
    let resumed_totals = std::mem::take(&mut completed.totals);
    let resumed_games = resumed_totals.games;
    let (resumed_samples, resumed_visits) = (resumed_totals.samples, resumed_totals.visit_samples);
    let work = Work::new(run.games, run.grant.clone());
    let totals = Mutex::new(resumed_totals);
    let runaway_lock = Mutex::new(());
    let started = Instant::now();
    let load = Arc::new(load);
    let load_opponent = &load_opponent;
    let governor = Governor::new(run.govern.clone(), run.lanes, run.limit);
    let governor = &governor;
    let status_done = AtomicBool::new(false);
    std::thread::scope(|scope| -> Result<(), String> {
        let monitor = std::thread::Builder::new()
            .name("selfplay-governor".into())
            .spawn_scoped(scope, move || governor.monitor())
            .map_err(|e| e.to_string())?;
        // However this ends (a lane that cannot be spawned included), the monitor ends and every lane is let go.
        let finishing = FinishOnDrop(governor, monitor.thread().clone());
        let (status_done, status_work, status_totals) = (&status_done, &work, &totals);
        let status = std::thread::Builder::new()
            .name("selfplay-status".into())
            .spawn_scoped(scope, move || {
                while !status_done.load(Ordering::SeqCst) {
                    write_status(run, &started, resumed_games, status_totals, status_work, governor, false);
                    std::thread::park_timeout(STATUS_EVERY);
                }
            })
            .map_err(|e| e.to_string())?;
        let status_thread = status.thread().clone();
        let _status_ends = StopOnDrop(status_done, status_thread);
        let mut lanes = Vec::new();
        for lane in 0..run.lanes.max(1) {
            let header = header.clone();
            let (work, totals, load, completed, runaway_lock) = (&work, &totals, Arc::clone(&load), &completed, &runaway_lock);
            let worker = std::thread::Builder::new().name(format!("selfplay-{lane}")).stack_size(STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                // However the lane ends, no other lane waits on it for a game: a lane that fails ends the run's
                // admissions, and the games in flight finish into their shards.
                let _closes = CloseOnExit(governor);
                // The models are loaded at the lane's first game, so a lane the governor never lets play holds none.
                let mut models: Option<(Box<dyn Evaluator>, Vec<Box<dyn Evaluator>>, Option<TractEvaluator>)> = None;
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
                    // Admitted before the game is chosen: which game is which is the index's alone.
                    // A lane that waits is no client of a batching evaluator meanwhile (`Evaluator::idle`).
                    let idle = || {
                        if let Some((evaluator, opponents, _)) = &models {
                            evaluator.idle();
                            opponents.iter().for_each(|o| o.idle());
                        }
                    };
                    let Some(ticket) = governor.admit(lane, idle) else { break };
                    let i = match work.take(|i| completed.done(i))? {
                        Pick::Game(i) => i,
                        Pick::Done => {
                            governor.close();
                            break;
                        }
                        // Nothing granted yet: the slot given back, the lane idle, and asked again in a moment.
                        Pick::Wait => {
                            drop(ticket);
                            idle();
                            let until = Instant::now() + GRANT_POLL;
                            while Instant::now() < until && governor.stopped().is_none() {
                                std::thread::sleep(Duration::from_millis(50));
                            }
                            continue;
                        }
                    };
                    if models.is_none() {
                        let loaded = governor.loading(|| -> Result<_, String> {
                            let evaluator = load()?;
                            let opponents = (0..options.opponents.len()).map(load_opponent).collect::<Result<Vec<_>, String>>()?;
                            let belief = match &options.belief {
                                Some(dir) => Some(TractEvaluator::load(dir, table()).map_err(|e| format!("the belief model {}: {e}", dir.display()))?),
                                None => None,
                            };
                            Ok((evaluator, opponents, belief))
                        });
                        match loaded {
                            Ok(m) => models = Some(m),
                            // After a stop (an inference server going down with the console's Ctrl-C, say), part of
                            // the stop, not a failure.
                            Err(_) if governor.stopped().is_some() => break,
                            Err(e) => return Err(e),
                        }
                    }
                    let Some((evaluator, opponents, belief)) = models.as_ref() else { break };
                    let (p, swap, first) = schedule(i, run.pairings.len());
                    let mut pairing = run.pairings[p].clone();
                    if swap {
                        pairing.names.swap(0, 1);
                        pairing.lists.swap(0, 1);
                    }
                    let seed = run.seed + i as u64;
                    let (opponent, start) = assignment(i, run.seed, options);
                    let start = start.and_then(|k| options.starts.as_ref().map(|s| &s.list[k]));
                    if let Some(s) = start {
                        pairing = Pairing { names: [format!("start:{}", s.name), format!("start:{}", s.name)], lists: s.lists.clone() };
                    }
                    let mut players = Players::mirror(evaluator.as_ref(), &options.model);
                    players.abort = Some(governor.abort_flag(lane));
                    players.meter = Some(&governor.meter);
                    if let Some(b) = belief {
                        players.beliefs = [Some(b as &dyn Belief), Some(b as &dyn Belief)];
                    }
                    if let Some((j, learner_seat)) = opponent {
                        let other = 1 - learner_seat;
                        players.beliefs[other] = None;
                        players.evaluators[other] = opponents[j].as_ref();
                        players.learner[other] = false;
                        players.names[other] = options.opponents[j].name.clone();
                    }
                    let result = play_with(&players, &pairing, seed, first, start, options, prior);
                    // Stopped by the governor, or failed once a stop was asked for: either way unrecorded, and played
                    // again on a resume.
                    let stopped = matches!(&result, Err(e) if e == governor::STOPPED) || (result.is_err() && governor.stopped().is_some());
                    // A game the run's own shards hold, played again for an `Options::also` directory that lacks it.
                    let in_main = completed.main.contains(&i);
                    match result {
                        Ok(mut played) if played.runaway.is_some() => {
                            let why = played.runaway.take().unwrap_or_default();
                            let (turns, answers, secs) = (played.game.state.turn.number, played.answers, played.elapsed.as_secs_f64());
                            let line = json!({
                                "index": i,
                                "seed": seed,
                                "decks": pairing.names,
                                "why": why,
                                "turns": turns,
                                "answers": answers,
                                "seconds": secs,
                                "trace": played.trace.take(),
                            });
                            {
                                let _held = runaway_lock.lock().unwrap_or_else(|e| e.into_inner());
                                let path = run.out.join(RUNAWAYS);
                                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                                writeln!(f, "{line}").map_err(|e| format!("{}: {e}", path.display()))?;
                            }
                            eprintln!(
                                "selfplay: game {i} (seed {seed}, {}) is a runaway, {why}: ended after {answers} answers, {turns} turns, {secs:.0} s; not recorded, its trace in {RUNAWAYS}",
                                pairing.names.join(" vs ")
                            );
                            totals.lock().unwrap_or_else(|e| e.into_inner()).runaways += 1;
                        }
                        Ok(mut played) => {
                            let (answers, searched, sims, evals, fallbacks, secs) =
                                (played.answers, played.searched, played.simulations, played.evaluations, played.network_fallbacks, played.elapsed.as_secs_f64());
                            let (turns, capped) = (played.game.state.turn.number, played.ending.capped);
                            if !in_main && let (Some(w), Some(t)) = (&mut traces, played.trace.take()) {
                                let line = serde_json::to_string(&t).map_err(|e| e.to_string())?;
                                writeln!(w, "{line}").and_then(|_| w.flush()).map_err(|e| e.to_string())?;
                            }
                            let shapes = std::mem::take(&mut played.shapes);
                            let snapshot_agreement = std::mem::take(&mut played.snapshots);
                            let belief_errors = played.belief_errors;
                            let timing = played.timing;
                            let full_searched = played.full_searched;
                            let fusions = std::mem::take(&mut played.fusions);
                            let winner = match played.ending.outcome {
                                Outcome::Won(w) if !capped => Some(w.index()),
                                _ => None,
                            };
                            for (k, (mode, w)) in also.iter_mut().enumerate() {
                                if completed.also[k].contains(&i) {
                                    continue;
                                }
                                let (rec, _) = record_searched(played.game.clone(), &played.dealt, &played.ending, *mode, table(), Some(&played.roots));
                                w.write(&rec).map_err(|e| e.to_string())?;
                            }
                            if in_main {
                                ticket.finish(true);
                                continue;
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
                            if let Some((j, learner_seat)) = opponent {
                                let o = t.opponents.entry(options.opponents[j].name.clone()).or_default();
                                o[0] += 1;
                                o[1] += u64::from(winner == Some(learner_seat));
                                o[2] += u64::from(winner.is_some());
                            }
                            if let Some(s) = start {
                                *t.starts.entry(s.name.clone()).or_default() += 1;
                            }
                            t.belief_errors += u64::from(belief_errors);
                            for (sims, agree) in snapshot_agreement {
                                let e = t.snapshots.entry(sims).or_default();
                                e[0] += 1;
                                e[1] += u64::from(agree);
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
                        // Stopped by the governor at a decision: not recorded, not a failure; a resumed run plays it.
                        Err(_) if stopped => {
                            totals.lock().unwrap_or_else(|e| e.into_inner()).stopped += 1;
                        }
                        Err(e) => {
                            let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                            t.dropped += 1;
                            let key: String = e.chars().take(120).collect();
                            *t.errors.entry(key).or_default() += 1;
                        }
                    }
                    ticket.finish(!stopped);
                }
                // Every lane's open shard is closed here, partly full or not (and by the writer's drop on an error).
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
        drop(finishing);
        let _ = monitor.join();
        first_error.map_or(Ok(()), Err)
    })?;
    write_status(run, &started, resumed_games, &totals, &work, governor, true);
    let stopped = governor.stopped();
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
        "FullBatch": options.full_batch,
        "FullSimulations": options.full_simulations,
        "FullShare": options.full_share,
        "FullSearched": t.full_searched,
    });
    let opponents: serde_json::Map<String, Value> = t
        .opponents
        .iter()
        .map(|(name, [games, wins, decided])| {
            let share = options.opponents.iter().find(|o| &o.name == name).map_or(0.0, |o| o.share);
            (name.clone(), json!({
                "Share": share,
                "GamesThisRun": games,
                "LearnerWins": wins,
                "Decided": decided,
                "LearnerWinRate": if *decided > 0 { *wins as f64 / *decided as f64 } else { 0.0 },
            }))
        })
        .collect();
    // How often each budget snapshot of a search already names the finished search's choice (`snapshot_agreement`), by
    // the count asked: null where no search kept snapshots (`Snapshots=...` not given).
    let snapshot_report = (!t.snapshots.is_empty()).then(|| {
        t.snapshots
            .iter()
            .map(|(sims, [n, agree])| {
                (sims.to_string(), json!({ "Searches": n, "Agree": agree, "AgreeShare": *agree as f64 / (*n).max(1) as f64 }))
            })
            .collect::<serde_json::Map<String, Value>>()
    });
    let report = json!({
        "Dir": run.out.display().to_string(),
        "Host": run.host,
        "Grant": run.grant.as_ref().map(|g| {
            let (granted, taken, ended) = work.counts();
            json!({ "File": g.display().to_string(), "Granted": granted, "Taken": taken, "Ended": ended })
        }),
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
        "Runaways": t.runaways,
        "MaxTurnAnswers": options.max_turn_answers,
        "MaxGameSeconds": options.max_game_seconds,
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
        "Opponents": opponents,
        "Starts": json!({
            "Dir": options.starts.as_ref().map(|s| s.dir.display().to_string()),
            "Share": options.start_share,
            "GamesThisRun": t.starts,
        }),
        "Snapshots": snapshot_report,
        "Belief": json!({
            "Model": options.belief.as_ref().map(|d| d.display().to_string()),
            "ErrorsThisRun": t.belief_errors,
        }),
        // Why the governor stopped the run (null: it was not stopped); its finished games are in closed shards, and
        // `--resume` plays the rest. The binary exits `governor::EXIT_STOPPED` then.
        "Stopped": stopped.as_ref().map(|why| json!({
            "Reason": why,
            "GamesStopped": t.stopped,
            "ExitCode": governor::EXIT_STOPPED,
        })),
        "Governor": governor.report(),
    });
    let _ = std::fs::write(
        run.out.join("selfplay-report.json"),
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
    if let Some(why) = &stopped {
        eprintln!(
            "selfplay: stopped ({why}): {} games of {} are in closed shards, {} stopped in flight were dropped; run again with --resume for the rest",
            t.games,
            run.games,
            t.stopped
        );
    }
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
