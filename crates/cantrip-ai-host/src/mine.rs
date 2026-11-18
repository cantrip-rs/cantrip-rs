//! The pair miner: training targets from the games a duel played in pairs (one deal, the players exchanging seats, each
//! deck staying with its seat), where a deep search with hidden information, not the games' outcomes, says which move is
//! better. Outcomes only choose which games are looked at.
//!
//! - `forks`: the two games of a pair walked in step from their shared deal to the first answer where they differ. Up
//!   to there the games are one game (each answer the same, so each position the same), so the fork is one position
//!   with two candidate answers, one from each player. The identity is checked, not assumed: at every answer before the
//!   fork the two games ask the same request, and at the fork their logs, boards and every encoded position the recorder
//!   wrote for either seat up to there are the same.
//! - `values`: a seat's network value at each of its decisions in a recorded game, by the model that played it: what
//!   the loser's largest value drop is found from (as `cantrip-reanalyse` finds it from root values).
//! - `judge`: a position's two candidate answers searched deeply over many worlds with a floor on both
//!   (`search::Config::favour_paths`, at every sub-step down to where they part), their values compared world by world
//!   (the same deal for both: a paired comparison) with a 95% interval, as `explore::verify` compares one nominated
//!   action with the best other. And `run`: a plan of positions (forks and loser decisions), each judged where it has
//!   candidates, each searched again without a floor and written as a policy target (`reanalyse::reanalyse_game`, so the
//!   shards are reanalyse's: meta `reanalysed`, policy targets alone), with the deep search's target against the
//!   network's prior (KL, top move) reported.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use cantrip_ai::cancel::board_hash;
use cantrip_ai::net::{Decision, Payment};
use cantrip_ai::record::table;
use cantrip_ai::search::{self, Config, WallClock};
use cantrip_ai::shard::{self, GameRecord, Header, ShardWriter};
use cantrip_ai::trace::{Seats, Step, Trace};
use cantrip_ai::{Answering, Evaluator, Knowledge, Prior, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::encode::from_compact;
use cantrip_obs::factor::{Token, decompose};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::explore::mean_interval;
use crate::reanalyse::{self, Chosen};

/// A recorded directory (the native host's `--ai-record` with `--ai-record-traces`): its games and traces by seed. Games
/// with no answers (a lane's empty games, BotLab's `--game-ids`) are left out.
pub struct Recorded {
    pub games: HashMap<u64, GameRecord>,
    pub traces: HashMap<u64, Trace>,
    pub mode: InfoMode,
}

pub fn load(dir: &Path) -> Result<Recorded, String> {
    let mut games = HashMap::new();
    let mut mode = None;
    for path in shard::shards(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let (header, records) = shard::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let m = shard::parse_mode(&header.mode).ok_or(format!("{}: mode {}", path.display(), header.mode))?;
        if mode.is_some_and(|x| x != m) {
            return Err(format!("{}: mode {}, not the other shards'", path.display(), header.mode));
        }
        mode = Some(m);
        for g in records.into_iter().filter(|g| g.meta.answers > 0) {
            if games.insert(g.meta.seed, g).is_some() {
                return Err(format!("{}: two games of seed {} (a directory holds one game a deal)", dir.display(), path.display()));
            }
        }
    }
    let traces = crate::regret::load_traces(&dir.join("traces"))?.into_iter().filter(|(_, t)| !t.steps.is_empty()).collect();
    Ok(Recorded { games, traces, mode: mode.ok_or(format!("{}: no shards", dir.display()))? })
}

/// The first sample with a choice of each decision of `game`, by decision.
fn firsts(game: &GameRecord) -> BTreeMap<u32, &shard::Sample> {
    let mut out: BTreeMap<u32, &shard::Sample> = BTreeMap::new();
    for s in &game.samples {
        if !out.contains_key(&s.decision) && legal(s) > 1 {
            out.insert(s.decision, s);
        }
    }
    out
}

/// Tokens as words, one each (their `Debug`), for JSON.
pub fn words(tokens: &[Token]) -> Vec<String> {
    tokens.iter().map(|t| format!("{t:?}")).collect()
}

fn legal(s: &shard::Sample) -> usize {
    from_compact(&s.position).map_or(0, |e| (0..e.options).filter(|&o| e.option_token(o).is_some_and(|t| e.legal[t])).count())
}

/// The tokens `answer` (the engine's, to `request`) is for the seat's search: `request` filtered as the seat's search
/// sees it (`Payment::filtered`), the answer factored in its terms and decomposed into the machine's tokens from its
/// start (`decompose`, with no names: as the search's machine has none). Also the filtered request's forced prefix.
fn tokens_of(game: &Game, payment: &Payment, request: &Request, answer: &Answer, mode: InfoMode) -> Result<(Vec<Token>, Vec<Token>), String> {
    let seat = request.player;
    let k = Knowledge::of(game, seat);
    let root = crate::explore::root(game, payment, &k, mode)?;
    let (obs, index) = observe_indexed(game, &k, seat, mode).ok_or("no observation of the request")?;
    let answering = Answering::new(game, &root.request, &obs, &index);
    let factored = answering.factored(&root.request, answer, None).ok_or("the answer has no factored form in the search's terms")?;
    let tokens = decompose(&answering.ask, &[], &factored)?;
    if !tokens.starts_with(&root.prefix) {
        return Err("the answer does not take the request's forced steps".into());
    }
    Ok((tokens, root.prefix))
}

// ---------------------------------------------------------------------------------------------------------------
// Forks

/// The fork of a pair's two games (`first`, `second`: each its trace and its recorded game), as a JSON object: `status`
/// "fork" (with the decision, the seat asked, the request's kind, both answers and the proof of identity), "identical"
/// (every answer the same: one game twice) or an error.
pub fn fork(first: (&Trace, &GameRecord), second: (&Trace, &GameRecord), mode: InfoMode) -> Result<Value, String> {
    let (ta, ra) = first;
    let (tb, rb) = second;
    if ta.seed != tb.seed || ta.lists != tb.lists {
        return Err("the two games are not one deal".into());
    }
    let mut ga = ta.dealt()?;
    let mut gb = tb.dealt()?;
    let mut sa = Seats::new(&ga);
    let mut sb = Seats::new(&gb);
    let n = ta.steps.len().min(tb.steps.len());
    let mut at = None;
    for i in 0..n {
        let qa = ga.pending().cloned().ok_or_else(|| format!("answer {i}: the first game asks nothing"))?;
        let qb = gb.pending().cloned().ok_or_else(|| format!("answer {i}: the second game asks nothing"))?;
        if qa != qb {
            return Err(format!("answer {i}: the games ask different requests with every answer before the same"));
        }
        let (x, y) = (&ta.steps[i], &tb.steps[i]);
        if x.seat != y.seat || x.answer != y.answer || x.by_hand != y.by_hand {
            at = Some(i);
            break;
        }
        let aa = sa.step(&ga, &qa, &ta.steps[i]).map_err(|e| format!("answer {i}: {e}"))?;
        let ab = sb.step(&gb, &qb, &tb.steps[i]).map_err(|e| format!("answer {i}: {e}"))?;
        if aa != ab {
            return Err(format!("answer {i}: one factored answer makes two engine answers"));
        }
        ga.answer(qa.id, qa.player, aa).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
        gb.answer(qb.id, qb.player, ab).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
    }
    let Some(i) = at else {
        let complete = ta.complete && tb.complete;
        return Ok(json!({
            "seed": ta.seed,
            "status": if ta.steps.len() == tb.steps.len() && complete { "identical" } else { "cut" },
            "answers": [ta.steps.len(), tb.steps.len()],
            "complete": [ta.complete, tb.complete],
        }));
    };
    // The identity at the fork: the same log (every answer), the same board, the same request, and every position the
    // recorder encoded for either seat before it (and at it) the same bytes in both games.
    let request = ga.pending().cloned().ok_or("the game asks nothing at the fork")?;
    let same_log = ga.log == gb.log;
    let same_board = board_hash(&ga.state) == board_hash(&gb.state);
    let same_request = gb.pending() == Some(&request);
    let encoded = |r: &GameRecord| -> Vec<(u32, u16, Vec<u8>)> {
        r.samples.iter().filter(|s| s.decision <= i as u32).map(|s| (s.decision, s.step, s.position.clone())).collect()
    };
    let (ea, eb) = (encoded(ra), encoded(rb));
    // At the fork itself the sub-steps after the first may follow the answers apart: compare the decisions before it whole,
    // and the fork's first sample.
    let before = |e: &[(u32, u16, Vec<u8>)]| e.iter().filter(|x| x.0 < i as u32).cloned().collect::<Vec<_>>();
    let first_at = |e: &[(u32, u16, Vec<u8>)]| e.iter().find(|x| x.0 == i as u32).cloned();
    let same_positions = before(&ea) == before(&eb);
    let same_fork_position = first_at(&ea).is_some() && first_at(&ea) == first_at(&eb);
    let both_ok = same_log && same_board && same_request && same_positions && same_fork_position;
    let aa = sa.step(&ga, &request, &ta.steps[i]).map_err(|e| format!("the fork, the first game's answer: {e}"))?;
    let ab = sb.step(&gb, &request, &tb.steps[i]).map_err(|e| format!("the fork, the second game's answer: {e}"))?;
    let payments = reanalyse::stand_in(ta, None, i)?.1;
    let payment = &payments[request.player.index()];
    let ka = tokens_of(&ga, payment, &request, &aa, mode);
    let kb = tokens_of(&ga, payment, &request, &ab, mode);
    let parted = match (&ka, &kb) {
        (Ok((x, prefix)), Ok((y, _))) => Some((x.iter().zip(y).take_while(|(a, b)| a == b).count(), prefix.len())),
        _ => None,
    };
    Ok(json!({
        "seed": ta.seed,
        "status": if both_ok { "fork" } else { "not-identical" },
        "decision": i,
        "seat": request.player.0,
        "turn": ga.state.turn.number,
        "kind": cantrip_ai::kind_name(&request.kind),
        "answers": [serde_json::to_value(&ta.steps[i]).unwrap_or_default(), serde_json::to_value(&tb.steps[i]).unwrap_or_default()],
        "labels": [crate::advisor::describe(&ga, &request, &aa), crate::advisor::describe(&ga, &request, &ab)],
        "tokens": [ka.as_ref().map(|k| json!(words(&k.0))).unwrap_or_else(|e| json!(format!("error: {e}"))), kb.as_ref().map(|k| json!(words(&k.0))).unwrap_or_else(|e| json!(format!("error: {e}")))],
        "parted_at": parted.map(|p| p.0),
        "prefix": parted.map(|p| p.1),
        "proof": {
            "same_log": same_log,
            "log": ga.log.len(),
            "same_board": same_board,
            "same_request": same_request,
            "same_positions": same_positions,
            "positions": before(&ea).len(),
            "same_fork_position": same_fork_position,
        },
        "outcomes": [ta.outcome, tb.outcome],
    }))
}

/// Every pair of `first` and `second` (by seed) forked (`fork`), a JSON line each into `out`, on `lanes` lanes; a
/// summary returned.
pub fn forks(first: &Recorded, second: &Recorded, lanes: usize, out: &Path) -> Result<Value, String> {
    let mut seeds: Vec<u64> = first.traces.keys().filter(|s| second.traces.contains_key(s)).copied().collect();
    seeds.sort_unstable();
    let next = AtomicUsize::new(0);
    let lines = Mutex::new(Vec::<Value>::new());
    std::thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::new();
        for lane in 0..lanes.max(1) {
            let (seeds, next, lines) = (&seeds, &next, &lines);
            let h = std::thread::Builder::new().name(format!("forks-{lane}")).stack_size(crate::STACK_SIZE).spawn_scoped(scope, move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&seed) = seeds.get(i) else { break };
                    let v = match (first.games.get(&seed), second.games.get(&seed)) {
                        (Some(ra), Some(rb)) => fork((&first.traces[&seed], ra), (&second.traces[&seed], rb), first.mode)
                            .unwrap_or_else(|e| json!({ "seed": seed, "status": "error", "error": e })),
                        _ => json!({ "seed": seed, "status": "error", "error": "a game has a trace but no recorded game" }),
                    };
                    lines.lock().unwrap_or_else(|e| e.into_inner()).push(v);
                }
            });
            handles.push(h.map_err(|e| e.to_string())?);
        }
        for h in handles {
            h.join().map_err(|_| "a lane panicked".to_string())?;
        }
        Ok(())
    })?;
    let mut lines = lines.into_inner().unwrap_or_else(|e| e.into_inner());
    lines.sort_by_key(|v| v["seed"].as_u64());
    std::fs::write(out, lines.iter().map(|v| format!("{v}\n")).collect::<String>()).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut by: BTreeMap<String, u64> = BTreeMap::new();
    for v in &lines {
        *by.entry(v["status"].as_str().unwrap_or("?").to_string()).or_default() += 1;
    }
    Ok(json!({ "Pairs": lines.len(), "ByStatus": by, "OnlyFirst": first.traces.len() - seeds.len(), "OnlySecond": second.traces.len() - seeds.len() }))
}

// ---------------------------------------------------------------------------------------------------------------
// Values

/// The decisions of `trace` at which `seat` (from 0) has a choice as its search sees the request: filtered as its
/// payment and cancel guard filter it (`Payment::filtered`, kept as self-play keeps them, `Payment::played`; the AI's
/// requests are atomic, so a priority whose only other options are mana abilities is a pass), with a first sub-step of
/// more than one option (`explore::root`), and no payment (`PayCosts`: the payment run's, never searched).
pub fn choices(trace: &Trace, seat: u8, mode: InfoMode) -> Result<std::collections::BTreeSet<u32>, String> {
    let mut game = trace.dealt()?;
    let mut seats = Seats::new(&game);
    let mut payments: Vec<Payment> = vec![Payment::default(); game.state.players.len()];
    let mut out = std::collections::BTreeSet::new();
    for (i, step) in trace.steps.iter().enumerate() {
        let request = game.pending().cloned().ok_or_else(|| format!("answer {i}: the game asks nothing"))?;
        let p = request.player.index();
        if request.player.0 == seat && step.kind != "PayCosts" {
            let k = Knowledge::of(&game, request.player);
            if crate::explore::root(&game, &payments[p], &k, mode).is_ok() {
                out.insert(i as u32);
            }
        }
        let answer = seats.step(&game, &request, step).map_err(|e| format!("answer {i}: {e}"))?;
        let board = board_hash(&game.state);
        payments[p].played(&game, board, &request, &answer);
        game.answer(request.id, request.player, answer).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
    }
    Ok(out)
}

/// The network value, for `seat` (from 0), of each of its decisions with a choice in `game` (its first such sub-step's
/// position as recorded): `(decision, turn, value)`. With `trace`, only the decisions where the seat's search has a
/// choice (`choices`).
pub fn values(game: &GameRecord, trace: Option<&Trace>, seat: u8, evaluator: &dyn Evaluator) -> Result<Vec<(u32, u16, f32)>, String> {
    let wanted = match trace {
        Some(t) => Some(choices(t, seat, InfoMode::Decklist)?),
        None => None,
    };
    let mine: Vec<(u32, &shard::Sample)> =
        firsts(game).into_iter().filter(|(d, s)| s.seat == seat && wanted.as_ref().is_none_or(|w| w.contains(d))).collect();
    let encoded: Vec<(u32, u16, cantrip_obs::encode::Encoded)> =
        mine.iter().filter_map(|(d, s)| from_compact(&s.position).map(|e| (*d, s.turn, e))).collect();
    if encoded.len() != mine.len() {
        return Err("a recorded position does not decode".into());
    }
    let mut out = Vec::new();
    for chunk in encoded.chunks(64) {
        let o = evaluator.evaluate_positions(&chunk.iter().map(|(_, _, e)| e).collect::<Vec<_>>()).map_err(|e| e.0)?;
        out.extend(chunk.iter().zip(o.value).map(|((d, t, _), v)| (*d, *t, v)));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------
// Judgement

/// How positions are judged and searched.
#[derive(Clone, Debug)]
pub struct JudgeOptions {
    pub simulations: usize,
    pub worlds: usize,
    pub batch: usize,
    /// The floor on each candidate answer, at every sub-step down to where they part.
    pub floor: f32,
    pub mode: InfoMode,
    pub seed: u64,
    /// More settings for the unfloored searches the targets come from (`search::Config::set`), as reanalyse takes them
    /// (`reanalyse::Options::search`): `RootShortcut=false`, as self-play's full searches run. The judgement's search
    /// already has no shortcut anywhere.
    pub search: Vec<(String, String)>,
}

impl Default for JudgeOptions {
    fn default() -> JudgeOptions {
        JudgeOptions { simulations: 4096, worlds: 16, batch: 32, floor: 0.2, mode: InfoMode::Decklist, seed: 0x6a1d6e, search: Vec::new() }
    }
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// The two candidate answers `candidates` (the engine's, to the request `game` waits on) judged (see the module): a
/// JSON object with each one's tokens, where they part, each world's values of the two at that sub-step, and the paired
/// difference (the first's less the second's) with its 95% interval and the verdict ("first", "second", "undecided").
pub fn judge(game: &Game, payment: &Payment, candidates: [&Answer; 2], evaluator: &dyn Evaluator, prior: &Prior, o: &JudgeOptions, seed: u64) -> Result<Value, String> {
    let request = game.pending().cloned().ok_or("the game asks nothing")?;
    let (ta, prefix) = tokens_of(game, payment, &request, candidates[0], o.mode)?;
    let (tb, _) = tokens_of(game, payment, &request, candidates[1], o.mode)?;
    if ta == tb {
        return Err("the two answers are one answer for the search".into());
    }
    let parted = ta.iter().zip(&tb).take_while(|(a, b)| a == b).count();
    let k = Knowledge::of(game, request.player);
    let config = Config {
        shortcut: 0.0,
        favour_paths: vec![(ta.clone(), o.floor), (tb.clone(), o.floor)],
        worlds: o.worlds,
        batch: o.batch.max(1),
        seed,
        report: true,
        ..Config::simulations(o.simulations)
    };
    let mode = o.mode;
    let d = payment.clone().decide(game, &request, &k, mode, |request, guard| {
        search::search(game, request, &k, mode, table(), evaluator, &config, prior, guard, &WallClock)
    });
    let d = d.map_err(|e| e.0)?;
    let s = d.search.as_ref().ok_or("the search made no answer of its own")?;
    let report = s.report.as_ref().ok_or("the search made no report")?;
    // Each world's two values at the node where the answers part: the deepest along both paths at or before the token
    // where they differ (a whole answer's edge (`Flat`) holds every token from its node, so there it is the root).
    let at_part = |path: &[search::FavouredAt]| path.iter().rev().find(|f| f.chosen.len() <= parted).map(|f| (f.chosen.len(), f.visits, f.value_sum));
    let mut diffs = Vec::new();
    let mut worlds = Vec::new();
    let (mut na, mut wa, mut nb, mut wb) = (0u32, 0f64, 0u32, 0f64);
    let mut node_depths = std::collections::BTreeSet::new();
    for w in &report.worlds {
        let (a, b) = match w.favoured.as_slice() {
            [a, b] => match (at_part(a), at_part(b)) {
                (Some(x), Some(y)) if x.0 == y.0 => {
                    node_depths.insert(x.0);
                    (Some((x.1, x.2)), Some((y.1, y.2)))
                }
                _ => (None, None),
            },
            _ => (None, None),
        };
        let q = |x: Option<(u32, f32)>| x.filter(|(n, _)| *n > 0).map(|(n, sum)| f64::from(sum) / f64::from(n));
        if let Some((n, sum)) = a {
            na += n;
            wa += f64::from(sum);
        }
        if let Some((n, sum)) = b {
            nb += n;
            wb += f64::from(sum);
        }
        if let (Some(x), Some(y)) = (q(a), q(b)) {
            diffs.push(x - y);
        }
        worlds.push(json!({ "dead": w.dead, "a": a.map(|x| x.0), "qa": q(a), "b": b.map(|x| x.0), "qb": q(b) }));
    }
    let (mean, lo, hi) = mean_interval(&diffs);
    let verdict = if diffs.len() < 2 {
        "undecided"
    } else if lo > 0.0 {
        "first"
    } else if hi < 0.0 {
        "second"
    } else {
        "undecided"
    };
    Ok(json!({
        "tokens": [words(&ta), words(&tb)],
        "prefix": prefix.len(),
        "parted_at": parted,
        "deeper": parted > prefix.len(),
        "compared_at": node_depths.into_iter().collect::<Vec<_>>(),
        "visits": [na, nb],
        "q": [if na > 0 { Some(wa / f64::from(na)) } else { None }, if nb > 0 { Some(wb / f64::from(nb)) } else { None }],
        "paired": diffs.len(),
        "difference": { "mean": mean, "low": lo, "high": hi },
        "verdict": verdict,
        "worlds": worlds,
        "root_value": s.root_value,
        "net_value": s.net_value,
        "simulations": s.simulations,
        "chosen": crate::advisor::describe(game, &request, &d.answer),
    }))
}

/// What a deep search (unfloored) made of a position, against the network's prior: the target at the root's first
/// sub-step with a choice (`Searched::visits`, as recorded), the prior there (the network's, `RootOption::net_prior`, by
/// first token), KL(target || prior), whether their top options agree, and the answer chosen, in tokens and words.
pub fn against_prior(game: &Game, payment: &Payment, d: &Decision, played: Option<&Answer>, mode: InfoMode) -> Value {
    let Some(s) = d.search.as_ref() else { return json!({ "error": "no search" }) };
    let request = match game.pending() {
        Some(r) => r.clone(),
        None => return json!({ "error": "the game asks nothing" }),
    };
    let first = s.visits.iter().find(|v| v.prefix == s.root_prefix);
    let mut prior: BTreeMap<Token, f32> = BTreeMap::new();
    for o in &s.root_options {
        if let Some(t) = o.path.first() {
            *prior.entry(*t).or_default() += o.net_prior;
        }
    }
    let (kl, top_agree, target) = match first {
        Some(f) => {
            let total: f64 = f.visits.iter().map(|&v| f64::from(v)).sum();
            let mut kl = 0.0;
            let mut target = Vec::new();
            for (t, &v) in f.tokens.iter().zip(&f.visits) {
                let p = f64::from(v) / total.max(1.0);
                let q = f64::from(prior.get(t).copied().unwrap_or(0.0)).max(1e-9);
                if p > 0.0 {
                    kl += p * (p / q).ln();
                }
                target.push(json!({ "token": format!("{t:?}"), "target": p, "prior": prior.get(t).copied().unwrap_or(0.0) }));
            }
            let best_target = (0..f.tokens.len()).max_by(|&a, &b| f.visits[a].cmp(&f.visits[b]).then(b.cmp(&a))).map(|k| f.tokens[k]);
            let best_prior = prior.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|(t, _)| *t);
            (Some(kl), best_target.is_some() && best_target == best_prior, target)
        }
        None => (None, false, Vec::new()),
    };
    let deep = tokens_of(game, payment, &request, &d.answer, mode).ok();
    let played_tokens = played.and_then(|a| tokens_of(game, payment, &request, a, mode).ok());
    let first_step = |t: &Option<(Vec<Token>, Vec<Token>)>| t.as_ref().and_then(|(t, p)| t.get(p.len()).copied());
    json!({
        "kl": kl,
        "top_agree": top_agree,
        "target": target,
        "options": s.root_options.len(),
        "root_value": s.root_value,
        "net_value": s.net_value,
        "chosen": crate::advisor::describe(game, &request, &d.answer),
        "chosen_tokens": deep.as_ref().map(|t| words(&t.0)),
        "played_tokens": played_tokens.as_ref().map(|t| words(&t.0)),
        "played_agree_first_step": match (first_step(&deep), first_step(&played_tokens)) {
            (Some(a), Some(b)) => Some(a == b),
            _ => None,
        },
        "played_agree": match (&deep, &played_tokens) {
            (Some(a), Some(b)) => Some(a.0 == b.0),
            _ => None,
        },
    })
}

/// One position of a plan (`run`).
#[derive(Clone, Debug, Deserialize)]
pub struct Planned {
    pub id: String,
    /// The recorded directory's name (`run`'s `dirs`).
    pub dir: String,
    pub seed: u64,
    pub decision: u32,
    /// "train" or "heldout": the shards it is written to.
    pub split: String,
    /// The two candidate answers, as traces keep them (a fork's: each game's step there); none for a position searched
    /// alone.
    #[serde(default)]
    pub candidates: Option<[Step; 2]>,
}

/// A run of judgements: the recorded directories by name, and where the output goes.
pub struct JudgeRun {
    pub dirs: BTreeMap<String, PathBuf>,
    pub out: PathBuf,
    pub lanes: usize,
    pub producer: String,
    /// Whether each position is also searched without a floor and written as a target (`reanalyse_game`).
    pub targets: bool,
}

/// The plan's positions judged and searched (see the module) on `run.lanes` lanes: a JSON line a position into
/// `OUT/judged.jsonl`, the targets' shards into `OUT/<split>/<dir>/` (`filter` then keeps the ones wanted), and a summary
/// returned (and written to `OUT/judge-report.json`).
pub fn run<F>(run: &JudgeRun, plan: &[Planned], o: &JudgeOptions, prior: &Prior, load: F) -> Result<Value, String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    std::fs::create_dir_all(&run.out).map_err(|e| format!("{}: {e}", run.out.display()))?;
    let judged_path = run.out.join("judged.jsonl");
    if judged_path.exists() {
        return Err(format!("{} exists; remove it", judged_path.display()));
    }
    let mut recorded: BTreeMap<String, Recorded> = BTreeMap::new();
    for (name, dir) in &run.dirs {
        let r = load_recorded(dir)?;
        if r.mode != o.mode {
            return Err(format!("{}: recorded in {} mode, judged in {}", dir.display(), shard::mode_name(r.mode), shard::mode_name(o.mode)));
        }
        recorded.insert(name.clone(), r);
    }
    let mut work: BTreeMap<(String, u64), Vec<&Planned>> = BTreeMap::new();
    for p in plan {
        if !recorded.contains_key(&p.dir) {
            return Err(format!("{}: no directory {}", p.id, p.dir));
        }
        work.entry((p.dir.clone(), p.seed)).or_default().push(p);
    }
    for list in work.values() {
        if list.iter().any(|p| p.split != list[0].split) {
            return Err(format!("game {} of {}: positions in two splits", list[0].seed, list[0].dir));
        }
    }
    let work: Vec<((String, u64), Vec<&Planned>)> = work.into_iter().collect();
    let mode = o.mode;
    let header = Header::new(table(), &run.producer, "mine", mode);
    let next = AtomicUsize::new(0);
    let lines = Mutex::new(Vec::<Value>::new());
    let totals = Mutex::new((0usize, 0usize, BTreeMap::<String, u64>::new()));
    std::thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::new();
        for lane in 0..run.lanes.max(1) {
            let (work, next, lines, totals, load, recorded, header) = (&work, &next, &lines, &totals, &load, &recorded, &header);
            let h = std::thread::Builder::new().name(format!("judge-{lane}")).stack_size(crate::STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                let evaluator = load()?;
                let mut writers: BTreeMap<String, ShardWriter> = BTreeMap::new();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(((dir, seed), planned)) = work.get(i) else { break };
                    let rec = &recorded[dir];
                    let (Some(trace), Some(game)) = (rec.traces.get(seed), rec.games.get(seed)) else {
                        let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                        *t.2.entry("no trace or game".into()).or_default() += planned.len() as u64;
                        continue;
                    };
                    let result = judge_game(trace, game, planned, run.targets, mode, evaluator.as_ref(), prior, o);
                    match result {
                        Ok((record, values, mismatched)) => {
                            let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                            t.1 += mismatched;
                            if let Some(rec) = record {
                                t.0 += rec.samples.len();
                                drop(t);
                                // By split and by recorded directory: two directories hold the two games of a pair,
                                // one deal (one seed) each, which `filter` tells apart by the directory.
                                let key = format!("{}/{dir}", planned[0].split);
                                if !writers.contains_key(&key) {
                                    let dir = run.out.join(&planned[0].split).join(dir);
                                    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                                    writers.insert(key.clone(), ShardWriter::new(&dir, &format!("mine-l{lane}"), header.clone(), 64 << 20, 100).map_err(|e| e.to_string())?);
                                }
                                writers.get_mut(&key).expect("made").write(&rec).map_err(|e| e.to_string())?;
                            }
                            lines.lock().unwrap_or_else(|e| e.into_inner()).extend(values);
                        }
                        Err(e) => {
                            let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                            *t.2.entry(e.chars().take(160).collect()).or_default() += planned.len() as u64;
                            drop(t);
                            let mut l = lines.lock().unwrap_or_else(|e| e.into_inner());
                            for p in planned {
                                l.push(json!({ "id": p.id, "error": e }));
                            }
                        }
                    }
                }
                for (_, mut w) in writers {
                    w.close().map_err(|e| e.to_string())?;
                }
                Ok(())
            });
            handles.push(h.map_err(|e| e.to_string())?);
        }
        for h in handles {
            h.join().map_err(|_| "a lane panicked".to_string())??;
        }
        Ok(())
    })?;
    let mut lines = lines.into_inner().unwrap_or_else(|e| e.into_inner());
    lines.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    std::fs::write(&judged_path, lines.iter().map(|v| format!("{v}\n")).collect::<String>()).map_err(|e| format!("{}: {e}", judged_path.display()))?;
    let (samples, mismatched, errors) = totals.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut verdicts: BTreeMap<String, u64> = BTreeMap::new();
    for l in &lines {
        if let Some(v) = l["judgement"]["verdict"].as_str() {
            *verdicts.entry(v.to_string()).or_default() += 1;
        }
    }
    let report = json!({
        "Positions": plan.len(),
        "Lines": lines.len(),
        "Samples": samples,
        "MismatchedPositions": mismatched,
        "Verdicts": verdicts,
        "Errors": errors,
        "Simulations": o.simulations,
        "Worlds": o.worlds,
        "Floor": o.floor,
        "Seed": o.seed,
        "Producer": run.producer,
        "Targets": run.targets,
    });
    let _ = std::fs::write(run.out.join("judge-report.json"), serde_json::to_string_pretty(&report).unwrap_or_default());
    Ok(report)
}

fn load_recorded(dir: &Path) -> Result<Recorded, String> {
    load(dir)
}

/// One game's planned positions: each fork judged, each position searched again and written (with `targets`), and a
/// JSON line each. Also the positions whose replay is not the recorded game (none written for the game then).
#[allow(clippy::too_many_arguments)]
fn judge_game(
    trace: &Trace,
    game: &GameRecord,
    planned: &[&Planned],
    targets: bool,
    mode: InfoMode,
    evaluator: &dyn Evaluator,
    prior: &Prior,
    o: &JudgeOptions,
) -> Result<(Option<GameRecord>, Vec<Value>, usize), String> {
    let firsts = firsts(game);
    let chosen: Vec<Chosen> = planned
        .iter()
        .map(|p| {
            let s = firsts.get(&p.decision).ok_or(format!("{}: decision {} has no choice in the recorded game", p.id, p.decision))?;
            Ok(Chosen {
                seed: trace.seed,
                decision: p.decision,
                reasons: vec!["mine"],
                step: s.step,
                recorded: s.visits.iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).map_or(0, |v| v.0),
                position: s.position.clone(),
            })
        })
        .collect::<Result<_, String>>()?;
    let reanalysed = if targets {
        let options = reanalyse::Options {
            simulations: o.simulations,
            worlds: o.worlds,
            batch: o.batch,
            seed: o.seed,
            search: o.search.clone(),
            ..reanalyse::Options::default()
        };
        let refs: Vec<&Chosen> = chosen.iter().collect();
        Some(reanalyse::reanalyse_game(trace, None, game, &refs, mode, evaluator, prior, &options)?)
    } else {
        None
    };
    let mut lines = Vec::new();
    for p in planned {
        let at = p.decision as usize;
        let (g, mut seats) = trace.replay(at)?;
        let (_, payments) = reanalyse::stand_in(trace, None, at)?;
        let request = g.pending().cloned().ok_or("the game asks nothing")?;
        let payment = &payments[request.player.index()];
        let played = trace.steps.get(at).and_then(|s| seats.step(&g, &request, s).ok());
        let mut line = json!({
            "id": p.id,
            "seed": trace.seed,
            "decision": p.decision,
            "seat": request.player.0,
            "turn": g.state.turn.number,
            "kind": cantrip_ai::kind_name(&request.kind),
            "split": p.split,
            "played": played.as_ref().map(|a| crate::advisor::describe(&g, &request, a)),
        });
        if let Some(r) = &reanalysed
            && let Some((_, d)) = r.decisions.iter().find(|(dd, _)| *dd == p.decision)
        {
            line["target"] = against_prior(&g, payment, d, played.as_ref(), mode);
        }
        if let Some([a, b]) = &p.candidates {
            let judged = (|| -> Result<Value, String> {
                let ea = seats.step(&g, &request, a)?;
                let eb = seats.step(&g, &request, b)?;
                let seed = splitmix(o.seed ^ trace.seed.rotate_left(13) ^ (u64::from(p.decision) << 24) ^ 0x4a);
                let mut v = judge(&g, payment, [&ea, &eb], evaluator, prior, o, seed)?;
                v["labels"] = json!([crate::advisor::describe(&g, &request, &ea), crate::advisor::describe(&g, &request, &eb)]);
                Ok(v)
            })();
            line["judgement"] = judged.unwrap_or_else(|e| json!({ "verdict": "unjudged", "error": e }));
        }
        lines.push(line);
    }
    let (record, mismatched) = match reanalysed {
        Some(r) => (r.record, r.mismatched),
        None => (None, 0),
    };
    Ok((record, lines, mismatched))
}

/// The positions `keep` names (`(dir, seed, decision)`) of the target shards `run` wrote under `from` (`from/<dir>/`, one
/// split), written into `out`: each game with only the kept decisions' samples and root values; a game none of whose
/// decisions is kept is left out. Returns the games and decisions written.
pub fn filter(from: &Path, keep: &std::collections::HashSet<(String, u64, u32)>, out: &Path, producer: &str) -> Result<(usize, usize), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    if !shard::shards(out).map_err(|e| e.to_string())?.is_empty() {
        return Err(format!("{} already has shards; remove them", out.display()));
    }
    let mut names: Vec<PathBuf> = std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    names.sort();
    let mut writer: Option<ShardWriter> = None;
    let (mut games, mut decisions) = (0usize, 0usize);
    for dir in names {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        for path in shard::shards(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let (header, records) = shard::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            for mut g in records {
                let kept = |d: u32| keep.contains(&(name.clone(), g.meta.seed, d));
                g.samples.retain(|s| kept(s.decision));
                g.meta.root_values.retain(|&(d, _)| kept(d));
                if g.samples.is_empty() {
                    continue;
                }
                if writer.is_none() {
                    let mode = shard::parse_mode(&header.mode).ok_or(format!("{}: mode {}", path.display(), header.mode))?;
                    writer = Some(ShardWriter::new(out, "mine", Header::new(table(), producer, "mine", mode), 64 << 20, 200).map_err(|e| e.to_string())?);
                }
                games += 1;
                decisions += g.meta.root_values.len();
                writer.as_mut().expect("made").write(&g).map_err(|e| e.to_string())?;
            }
        }
    }
    if let Some(mut w) = writer {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok((games, decisions))
}
