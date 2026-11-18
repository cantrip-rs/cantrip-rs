//! Reanalyse (the review of 2026-11-13, item 8; MuZero's Reanalyse): self-played games' chosen positions searched
//! again, deeply and in bulk, by the newest network, and written as shards of search targets. Self-play's full searches
//! fall where a coin puts them and mostly confirm the policy; reanalyse puts the deep searches where they are likely to
//! disagree with it.
//!
//! A game is stood in again from its trace (`cantrip_ai::trace`: self-play's `--traces`), answer by answer, each
//! seat's payment state and cancel guard kept as self-play kept them (`net::Payment::played`), so a chosen request is
//! the one the game asked, filtered alike. Its positions are chosen from its shard (`select`):
//!
//! - `fast-differs`: a fast search (playout-cap randomisation's, `GameMeta::fast_decisions`) whose most visited option
//!   is not the network's (the newest one's) likeliest;
//! - `flat`: the network's likeliest option has less than `Options::flat` of its prior;
//! - `swing`: the seat's root value moved by `Options::swing` or more from its previous searched decision in the turn
//!   (both decisions are chosen);
//! - `loser`: the losing seat's `Options::before_swing` searched decisions up to the largest fall of its root value in
//!   the game;
//! - and a share `Options::random_share` of all the positions, drawn uniformly from every one that could be chosen, so
//!   the choice is not all bias.
//!
//! Only the learner's decisions (`GameMeta::learners`) whose first sub-step has a choice, and that no full search
//! answered already, are candidates; the prior criteria need the shard's card index to be this build's (else only the
//! value criteria and the random share choose). Positions with more reasons go first.
//!
//! Each chosen position is searched afresh (no kept tree) at `Options::simulations` over `Options::worlds`, and the
//! game is recorded (`cantrip_ai::record`) with those roots alone: the samples of the chosen decisions are written, their
//! labels the game's own (outcome, life swing), their visits and q the deep search's, their decisions in the meta's
//! `root_values`, and the meta says `reanalysed`. A replayed position whose encoding is not the one the shard holds
//! means the trace is not the game: the game's positions are dropped and counted.
//!
//! The imitation corpus (the native host's `--ai-record`) is reanalysed alike when it was recorded with traces
//! (`--ai-record-traces`). Such a trace is made from the finished game (`Trace::of_game`) and may be cut short by a cost
//! paid by hand: only the decisions before the cut are candidates, and their samples keep the shard's labels.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use cantrip_ai::cancel::board_hash;
use cantrip_ai::net::{Choice, Decision, Payment};
use cantrip_ai::record::{Dealt, Ending, Root, Roots, record_searched, table};
use cantrip_ai::search::{self, Config, WallClock};
use cantrip_ai::shard::{self, GameRecord, Header, ShardWriter, parse_mode};
use cantrip_ai::trace::{Seats, Trace};
use cantrip_ai::{Evaluator, Knowledge, Prior};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::encode::from_compact;
use serde_json::{Value, json};

use crate::STACK_SIZE;
use crate::selfplay::{Start, Starts};

/// How positions are chosen and searched.
#[derive(Clone, Debug)]
pub struct Options {
    /// Positions to search in all.
    pub positions: usize,
    /// The share of them drawn uniformly from every candidate.
    pub random_share: f32,
    /// A prior whose likeliest option holds less than this is flat.
    pub flat: f32,
    /// A move of the seat's root value this large within a turn is a swing.
    pub swing: f32,
    /// The losing seat's searched decisions taken up to its largest fall.
    pub before_swing: usize,
    pub simulations: usize,
    pub worlds: usize,
    pub batch: usize,
    pub seed: u64,
    /// More search settings by name (`search::Config::set`: `RootShortcut=false`, say), applied in order.
    pub search: Vec<(String, String)>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            positions: 1000,
            random_share: 0.25,
            flat: 0.6,
            swing: 0.3,
            before_swing: 3,
            simulations: 4096,
            worlds: 8,
            batch: 32,
            seed: 0x7ea9a1,
            search: Vec::new(),
        }
    }
}

/// A position chosen: its game (by seed) and decision, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Chosen {
    pub seed: u64,
    pub decision: u32,
    pub reasons: Vec<&'static str>,
    /// The decision's first sub-step with a choice (`Sample::step`), its recorded visits' most visited option, and its
    /// position (compact).
    pub step: u16,
    pub recorded: u16,
    pub position: Vec<u8>,
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn argmax_visits(visits: &[(u16, f32)]) -> u16 {
    visits.iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).map_or(0, |v| v.0)
}

/// The legal options of a sample's position.
fn legal(s: &shard::Sample) -> usize {
    from_compact(&s.position).map_or(0, |e| (0..e.options).filter(|&o| e.option_token(o).is_some_and(|t| e.legal[t])).count())
}

/// A game's candidates: each learner decision with a choice not answered by a full search, the sample of its first
/// sub-step with a choice, and the reasons the game's labels give (`swing`, `loser`).
fn candidates<'a>(game: &'a GameRecord, options: &Options) -> Vec<(u32, &'a shard::Sample, Vec<&'static str>)> {
    let meta = &game.meta;
    let learner = |seat: u8| meta.learners.is_empty() || meta.learners.contains(&(seat + 1));
    let fast: HashSet<u32> = meta.fast_decisions.iter().copied().collect();
    let roots: BTreeMap<u32, f32> = meta.root_values.iter().filter(|(_, v)| v.is_finite()).copied().collect();
    let mut firsts: BTreeMap<u32, &shard::Sample> = BTreeMap::new();
    for s in &game.samples {
        if !firsts.contains_key(&s.decision) && legal(s) > 1 {
            firsts.insert(s.decision, s);
        }
    }
    let mut reasons: BTreeMap<u32, Vec<&'static str>> = BTreeMap::new();
    // Swings: each seat's root values in order, within a turn.
    for seat in 0..2u8 {
        let mine: Vec<(u32, u16, f32)> = roots.iter().filter_map(|(&d, &v)| firsts.get(&d).filter(|s| s.seat == seat).map(|s| (d, s.turn, v))).collect();
        for w in mine.windows(2) {
            if w[0].1 == w[1].1 && (w[1].2 - w[0].2).abs() >= options.swing {
                reasons.entry(w[0].0).or_default().push("swing");
                reasons.entry(w[1].0).or_default().push("swing");
            }
        }
        // The loser's decisions up to its largest fall.
        if firsts.values().any(|s| s.seat == seat && s.outcome < 0)
            && let Some(k) = (1..mine.len()).max_by(|&a, &b| (mine[a - 1].2 - mine[a].2).total_cmp(&(mine[b - 1].2 - mine[b].2)))
            && mine[k - 1].2 > mine[k].2
        {
            for &(d, _, _) in &mine[k.saturating_sub(options.before_swing)..k] {
                reasons.entry(d).or_default().push("loser");
            }
        }
    }
    firsts
        .iter()
        .filter(|(d, s)| learner(s.seat) && (!roots.contains_key(d) || fast.contains(d)))
        .map(|(&d, &s)| {
            let mut r = reasons.remove(&d).unwrap_or_default();
            r.dedup();
            (d, s, r)
        })
        .collect()
}

/// The positions chosen from `games` (each a shard's header and its games): see the module. `evaluator` is the newest
/// network, asked for its prior at each candidate whose shard's card index is this build's.
pub fn select(games: &[(Header, Vec<GameRecord>)], evaluator: &dyn Evaluator, options: &Options) -> Result<Vec<Chosen>, String> {
    let mut all: Vec<Chosen> = Vec::new();
    for (header, records) in games {
        let same_index = header.text_hash == table().hash;
        for g in records {
            let fast: HashSet<u32> = g.meta.fast_decisions.iter().copied().collect();
            let cands = candidates(g, options);
            let mut priors: Vec<Option<Vec<f32>>> = vec![None; cands.len()];
            if same_index {
                let encoded: Vec<_> = cands.iter().map(|(_, s, _)| from_compact(&s.position)).collect();
                let present: Vec<(usize, &cantrip_obs::encode::Encoded)> = encoded.iter().enumerate().filter_map(|(i, e)| e.as_ref().map(|e| (i, e))).collect();
                for chunk in present.chunks(64) {
                    let out = evaluator.evaluate_positions(&chunk.iter().map(|(_, e)| *e).collect::<Vec<_>>()).map_err(|e| e.0)?;
                    for (k, (i, _)) in chunk.iter().enumerate() {
                        priors[*i] = Some(out.policy[k].clone());
                    }
                }
            }
            for ((d, s, mut reasons), prior) in cands.into_iter().zip(priors) {
                if let Some(logits) = prior {
                    let e = from_compact(&s.position);
                    let legal: Vec<usize> = e.as_ref().map_or(Vec::new(), |e| (0..e.options).filter(|&o| e.option_token(o).is_some_and(|t| e.legal[t])).collect());
                    let top = legal.iter().map(|&o| logits.get(o).copied().unwrap_or(f32::MIN)).fold(f32::MIN, f32::max);
                    let sum: f32 = legal.iter().map(|&o| (logits.get(o).copied().unwrap_or(f32::MIN) - top).exp()).sum();
                    let best = legal.iter().copied().max_by(|&a, &b| logits.get(a).copied().unwrap_or(f32::MIN).total_cmp(&logits.get(b).copied().unwrap_or(f32::MIN)).then(b.cmp(&a)));
                    if sum > 0.0 && 1.0 / sum < options.flat {
                        reasons.push("flat");
                    }
                    if fast.contains(&d) && s.searched && best.is_some_and(|b| b as u16 != argmax_visits(&s.visits)) {
                        reasons.push("fast-differs");
                    }
                }
                all.push(Chosen { seed: g.meta.seed, decision: d, reasons, step: s.step, recorded: argmax_visits(&s.visits), position: s.position.clone() });
            }
        }
    }
    // The random share first, from every candidate; then the rest by reasons, ties by a hash of the seed.
    let key = |c: &Chosen| splitmix(options.seed ^ c.seed.rotate_left(17) ^ u64::from(c.decision));
    let random = ((options.positions as f64) * f64::from(options.random_share)).round() as usize;
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by_key(|&i| key(&all[i]));
    let mut taken: Vec<bool> = vec![false; all.len()];
    let mut out = Vec::new();
    for &i in order.iter().take(random.min(options.positions)) {
        taken[i] = true;
        let mut c = all[i].clone();
        c.reasons.push("random");
        out.push(c);
    }
    let mut rest: Vec<usize> = (0..all.len()).filter(|&i| !taken[i] && !all[i].reasons.is_empty()).collect();
    rest.sort_by(|&a, &b| all[b].reasons.len().cmp(&all[a].reasons.len()).then(key(&all[a]).cmp(&key(&all[b]))));
    for i in rest.into_iter().take(options.positions.saturating_sub(out.len())) {
        out.push(all[i].clone());
    }
    out.sort_by_key(|c| (c.seed, c.decision));
    Ok(out)
}

/// What one game's reanalysis made.
pub struct Reanalysed {
    pub record: Option<GameRecord>,
    pub searched: usize,
    /// Positions whose deep search's most visited first option is the recorded one.
    pub agree: usize,
    /// Positions the replay could not stand in (another encoding than the shard's), dropped with the game.
    pub mismatched: usize,
    pub simulations: u64,
    /// Each chosen position's deep search, by its decision: for a caller reporting on the searches beside the shards
    /// (`cantrip-explore mine`).
    pub decisions: Vec<(u32, Decision)>,
}

/// The game of `trace` stood in again (from `start`'s position where it began at one), each position of `chosen`
/// searched at it (see the module), and the game recorded with those roots alone. `original` is the game as its shard
/// holds it: where the trace is cut short (a table's game, `Trace::of_game`), the replay ends before the game did, and
/// each sample's labels (the outcome, the life swing, death by the next turn, the opponents' hands) and answer are the
/// original's, as is the meta's ending.
#[allow(clippy::too_many_arguments)]
pub fn reanalyse_game(
    trace: &Trace,
    start: Option<&Start>,
    original: &GameRecord,
    chosen: &[&Chosen],
    mode: InfoMode,
    evaluator: &dyn Evaluator,
    prior: &Prior,
    options: &Options,
) -> Result<Reanalysed, String> {
    let meta = &original.meta;
    let mut game = match start {
        Some(s) => s.game_for(trace.seed),
        None => trace.dealt()?,
    };
    let start_node = game.at;
    let mut seats = Seats::new(&game);
    let mut payments: Vec<Payment> = vec![Payment::default(); game.state.players.len()];
    let wanted: HashMap<u32, &Chosen> = chosen.iter().map(|c| (c.decision, *c)).collect();
    let mut roots = Roots::new();
    let mut out = Reanalysed { record: None, searched: 0, agree: 0, mismatched: 0, simulations: 0, decisions: Vec::new() };
    for (i, step) in trace.steps.iter().enumerate() {
        let request = game.pending().cloned().ok_or_else(|| format!("answer {i}: the game asks nothing"))?;
        let p = request.player.index();
        if wanted.contains_key(&(i as u32)) {
            let knowledge = Knowledge::of(&game, request.player);
            let mut config = Config {
                choice: Choice::Best,
                seed: splitmix(options.seed ^ trace.seed.rotate_left(13) ^ ((i as u64) << 24)),
                worlds: options.worlds,
                batch: options.batch.max(1),
                ..Config::simulations(options.simulations)
            };
            for (k, v) in &options.search {
                config.set(k, v)?;
            }
            let decided = payments[p].clone().decide(&game, &request, &knowledge, mode, |request, guard| {
                search::search(&game, request, &knowledge, mode, table(), evaluator, &config, prior, guard, &WallClock)
            });
            if let Ok(d) = decided
                && let Some(s) = d.search.as_ref().filter(|s| !s.visits.is_empty())
            {
                out.searched += 1;
                out.simulations += s.simulations as u64;
                roots.insert(game.at, Root::of(s, true));
                out.decisions.push((i as u32, d));
            }
        }
        let answer = seats.step(&game, &request, step).map_err(|e| format!("answer {i}: {e}"))?;
        let board = board_hash(&game.state);
        payments[p].played(&game, board, &request, &answer);
        game.answer(request.id, request.player, answer).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
    }
    let capped = !game.is_over();
    let ending = Ending::of(&game, capped);
    let dealt = Dealt {
        seed: meta.seed,
        names: meta.names.clone(),
        decks: meta.decks.clone(),
        auto_start: start.is_none(),
        learners: meta.learners.clone(),
        start: start.map(|s| (start_node, s.name.clone())),
        ..Dealt::default()
    };
    let (mut record, _) = record_searched(game, &dealt, &ending, mode, table(), Some(&roots));
    let keep: HashSet<u32> = record.meta.root_values.iter().map(|&(d, _)| d).collect();
    // The replay must be the game recorded: each chosen position's sub-step as the shard holds it. Where it is, the deep
    // search's most visited option is compared with the recorded one (both by the recorder's option index).
    for c in chosen {
        if !keep.contains(&c.decision) {
            continue;
        }
        match record.samples.iter().find(|s| s.decision == c.decision && s.step == c.step) {
            Some(s) if s.position == c.position => out.agree += usize::from(argmax_visits(&s.visits) == c.recorded),
            _ => out.mismatched += 1,
        }
    }
    if out.mismatched > 0 {
        return Ok(out);
    }
    record.samples.retain(|s| keep.contains(&s.decision));
    if !trace.complete {
        for s in &mut record.samples {
            match original.samples.iter().find(|o| o.decision == s.decision && o.step == s.step && o.position == s.position) {
                Some(o) => {
                    s.outcome = o.outcome;
                    s.life_swing = o.life_swing;
                    s.dead_next_turn = o.dead_next_turn;
                    s.opponent_hand = o.opponent_hand.clone();
                    s.chosen = o.chosen;
                }
                None => out.mismatched += 1,
            }
        }
        if out.mismatched > 0 {
            return Ok(out);
        }
        record.meta.ended = meta.ended.clone();
        record.meta.winner = meta.winner;
        record.meta.turns = meta.turns;
        record.meta.answers = meta.answers;
    }
    record.meta.reanalysed = true;
    out.record = (!record.samples.is_empty()).then_some(record);
    Ok(out)
}

/// The game of `trace` stood in again (from `start`'s position where it began at one) up to its answer `upto`, and
/// each seat's payment state and cancel guard as self-play kept them (`Payment::played`): a request there is filtered
/// as self-play filtered it (`Payment::filtered`, `Payment::decide`).
pub fn stand_in(trace: &Trace, start: Option<&Start>, upto: usize) -> Result<(Game, Vec<Payment>), String> {
    let mut game = match (&trace.start, start) {
        (Some(_), Some(s)) => s.game_for(trace.seed),
        (Some(name), None) => return Err(format!("the game began at scenario {name}: its position is needed (--starts)")),
        (None, _) => trace.dealt()?,
    };
    let mut seats = Seats::new(&game);
    let mut payments: Vec<Payment> = vec![Payment::default(); game.state.players.len()];
    for (i, step) in trace.steps.iter().take(upto).enumerate() {
        let request = game.pending().cloned().ok_or_else(|| format!("answer {i}: the game asks nothing"))?;
        let answer = seats.step(&game, &request, step).map_err(|e| format!("answer {i}: {e}"))?;
        let board = board_hash(&game.state);
        payments[request.player.index()].played(&game, board, &request, &answer);
        game.answer(request.id, request.player, answer).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
    }
    Ok((game, payments))
}

/// A run of reanalysis: where the self-played games are, and where the shards go.
pub struct Run {
    /// Self-play directories, each with its shards and its `traces/`.
    pub dirs: Vec<PathBuf>,
    pub out: PathBuf,
    pub lanes: usize,
    pub producer: String,
    /// The scenario positions the games from `--starts` began at.
    pub starts: Option<Starts>,
}

/// Chooses the positions, searches them on `run.lanes` lanes (an evaluator each from `load`), writes their shards
/// into `run.out` and a report (`reanalyse-report.json`, also returned) and the chosen positions
/// (`reanalyse-positions.jsonl`).
pub fn run<F>(run: &Run, options: &Options, prior: &Prior, load: F) -> Result<Value, String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    std::fs::create_dir_all(&run.out).map_err(|e| format!("{}: {e}", run.out.display()))?;
    if !shard::shards(&run.out).map_err(|e| e.to_string())?.is_empty() {
        return Err(format!("{} already has shards; remove them", run.out.display()));
    }
    let started = Instant::now();
    let mut games: Vec<(Header, Vec<GameRecord>)> = Vec::new();
    let mut traces: HashMap<u64, Trace> = HashMap::new();
    let mut mode: Option<InfoMode> = None;
    for dir in &run.dirs {
        for path in shard::shards(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let (header, records) = shard::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let m = parse_mode(&header.mode).ok_or(format!("{}: mode {}", path.display(), header.mode))?;
            if mode.is_some_and(|x| x != m) {
                return Err(format!("{}: mode {}, not the other shards'", path.display(), header.mode));
            }
            mode = Some(m);
            games.push((header, records));
        }
        let t = dir.join("traces");
        if t.is_dir() {
            traces.extend(crate::regret::load_traces(&t)?);
        }
    }
    let mode = mode.ok_or("no shards to reanalyse")?;
    // Only games with a trace; of a trace cut short (a table's game), only the decisions it holds are candidates (the
    // games themselves are kept whole: their samples give the labels, `reanalyse_game`).
    for (_, records) in &mut games {
        records.retain(|g| traces.get(&g.meta.seed).is_some_and(|t| t.start.as_deref().unwrap_or("") == g.meta.start));
    }
    let held: HashMap<u64, usize> = traces.iter().filter(|(_, t)| !t.complete).map(|(&s, t)| (s, t.steps.len())).collect();
    let candidates: Vec<(Header, Vec<GameRecord>)> = games
        .iter()
        .map(|(h, records)| {
            let records = records
                .iter()
                .map(|g| match held.get(&g.meta.seed) {
                    Some(&n) => GameRecord { meta: g.meta.clone(), samples: g.samples.iter().filter(|s| (s.decision as usize) < n).cloned().collect() },
                    None => g.clone(),
                })
                .collect();
            (h.clone(), records)
        })
        .collect();
    let traced: usize = games.iter().map(|(_, r)| r.len()).sum();
    let evaluator = load()?;
    let chosen = select(&candidates, evaluator.as_ref(), options)?;
    drop(candidates);
    drop(evaluator);
    let mut by_reason: BTreeMap<&str, u64> = BTreeMap::new();
    for c in &chosen {
        for r in &c.reasons {
            *by_reason.entry(r).or_default() += 1;
        }
    }
    {
        let path = run.out.join("reanalyse-positions.jsonl");
        let text: String = chosen.iter().map(|c| format!("{}\n", json!({ "seed": c.seed, "decision": c.decision, "reasons": c.reasons }))).collect();
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let metas: HashMap<u64, &GameRecord> = games.iter().flat_map(|(_, r)| r.iter().map(|g| (g.meta.seed, g))).collect();
    let mut work: BTreeMap<u64, Vec<&Chosen>> = BTreeMap::new();
    for c in &chosen {
        work.entry(c.seed).or_default().push(c);
    }
    let work: Vec<(u64, Vec<&Chosen>)> = work.into_iter().collect();
    let header = Header::new(table(), &run.producer, "reanalyse", mode);
    let next = AtomicUsize::new(0);
    let totals = Mutex::new((0usize, 0usize, 0usize, 0u64, 0usize, BTreeMap::<String, u64>::new()));
    std::thread::scope(|scope| -> Result<(), String> {
        let mut lanes = Vec::new();
        for lane in 0..run.lanes.max(1) {
            let (work, next, totals, load, traces, metas, header) = (&work, &next, &totals, &load, &traces, &metas, &header);
            let h = std::thread::Builder::new().name(format!("reanalyse-{lane}")).stack_size(STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                let evaluator = load()?;
                let mut writer = ShardWriter::new(&run.out, &format!("reanalyse-l{lane}"), header.clone(), 64 << 20, 100).map_err(|e| e.to_string())?;
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((seed, chosen)) = work.get(i) else { break };
                    let (Some(trace), Some(meta)) = (traces.get(seed), metas.get(seed)) else { continue };
                    let start = match &trace.start {
                        Some(name) => match run.starts.as_ref().and_then(|s| s.list.iter().find(|x| &x.name == name)) {
                            Some(s) => Some(s),
                            None => {
                                let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                                *t.5.entry(format!("no start {name} (--starts)")).or_default() += 1;
                                continue;
                            }
                        },
                        None => None,
                    };
                    let result = reanalyse_game(trace, start, meta, chosen, mode, evaluator.as_ref(), prior, options);
                    let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                    match result {
                        Ok(r) => {
                            t.0 += r.searched;
                            t.1 += r.agree;
                            t.2 += r.mismatched;
                            t.3 += r.simulations;
                            if let Some(rec) = &r.record {
                                t.4 += rec.samples.len();
                                drop(t);
                                writer.write(rec).map_err(|e| e.to_string())?;
                            }
                        }
                        Err(e) => *t.5.entry(e.chars().take(120).collect()).or_default() += 1,
                    }
                }
                writer.close().map_err(|e| e.to_string())
            });
            lanes.push(h.map_err(|e| e.to_string())?);
        }
        for h in lanes {
            h.join().map_err(|_| "a lane panicked".to_string())??;
        }
        Ok(())
    })?;
    let (searched, agree, mismatched, simulations, samples, errors) = totals.into_inner().unwrap_or_else(|e| e.into_inner());
    let report = json!({
        "Dirs": run.dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
        "Out": run.out.display().to_string(),
        "Producer": run.producer,
        "Mode": shard::mode_name(mode),
        "Simulations": options.simulations,
        "Worlds": options.worlds,
        "GamesRead": games.iter().map(|(_, r)| r.len()).sum::<usize>(),
        "GamesTraced": traced,
        "TracesCutShort": held.len(),
        "Chosen": chosen.len(),
        "ByReason": by_reason,
        "Searched": searched,
        "AgreeWithRecorded": agree,
        "AgreeShare": agree as f64 / searched.max(1) as f64,
        "MismatchedPositions": mismatched,
        "Samples": samples,
        "SimulationsRun": simulations,
        "Errors": errors,
        "Seconds": started.elapsed().as_secs_f64(),
    });
    let _ = std::fs::write(run.out.join("reanalyse-report.json"), serde_json::to_string_pretty(&report).unwrap_or_default());
    Ok(report)
}

/// The traces a self-play directory holds (its `traces/`), by seed: for a caller checking what can be reanalysed.
pub fn traced(dir: &Path) -> Result<HashMap<u64, Trace>, String> {
    crate::regret::load_traces(&dir.join("traces"))
}
