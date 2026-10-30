//! The regret measure's reference: what each of a position's candidate answers is worth
//! to the seat, as an expectation over the hidden game the seat cannot see and over the chance after it.
//!
//! A position is a request a traced game asked (`cantrip_ai::trace`), stood in again by replaying the trace. The seat's
//! candidates are whole answers: the answer the game's player gave, and for each of the network's `top` likeliest
//! options at the request's first choice, that option with the rest of the answer the network's best. Each is played in
//! `worlds` worlds dealt from the seat's knowledge (`cantrip_ai::determinize`, Decklist mode: the opponent's unseen cards
//! from their list less what the seat has seen, every library's unknown order anew), the same worlds for every
//! candidate, and the game continued from there by the network alone for both seats, each seat on its own Decklist
//! observation of the world (`net::answer_by` with the seat's own knowledge), so that neither continues knowing the
//! other's hand. A line ends at the game's end (1 won, -1 lost, 0 a draw the rules make) or `turns` turns past the
//! position, where the network's value of the next request is the line's (never 0: a line cut short is no draw). A
//! candidate's value is the mean over the worlds; the regret of an answer is the best candidate's value less its own.
//!
//! The reference is only as good as the network that continues the lines; its identity (model, worlds, turns) goes
//! with every number it makes.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use cantrip_ai::eval::{FeatureBatch, MASKED};
use cantrip_ai::net::{self, Choice, Payment};
use cantrip_ai::record::table;
use cantrip_ai::cancel::CancelGuard;
use cantrip_ai::issearch;
use cantrip_ai::search::{self, WallClock};
use cantrip_ai::trace::Trace;
use cantrip_ai::{Answering, Evaluator, Knowledge, Prior, determinize, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::{InfoMode, Observation};
use cantrip_obs::encode::{self, to_compact};
use cantrip_obs::factor::Machine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::STACK_SIZE;

/// How the reference is made.
#[derive(Clone, Debug)]
pub struct Options {
    /// Worlds a candidate is played in: dealt anew each, the hidden cards and every unknown library order.
    pub worlds: usize,
    /// The network's likeliest options at the first choice made into candidates.
    pub top: usize,
    /// Turns past the position a line is played before the network's value ends it.
    pub turns: u32,
    /// Answers in a line before the network's value ends it.
    pub max_answers: u32,
    pub seed: u64,
    /// Players whose answers at each position are scored too, each by a name.
    pub players: Vec<(String, Player)>,
}

impl Default for Options {
    fn default() -> Options {
        Options { worlds: 64, top: 4, turns: 10, max_answers: 4000, seed: 0x7e9e7, players: Vec::new() }
    }
}

/// A player scored at the positions: PIMC (`search::search`) or the information-set search (`issearch::search`), each
/// in Decklist mode with the reference's network.
#[derive(Clone, Debug)]
pub enum Player {
    Pimc(search::Config),
    Is(issearch::Config),
}

impl Player {
    /// `pimc:WORLDS:SIMULATIONS` (simulations over all worlds, as the PUCT search counts them), or
    /// `is:WORLDS:EVALUATIONS[:extend]`.
    pub fn parse(spec: &str) -> Result<Player, String> {
        let parts: Vec<&str> = spec.split(':').collect();
        let n = |i: usize| parts.get(i).and_then(|x| x.parse::<usize>().ok()).ok_or(format!("no player {spec}: pimc:W:SIMS or is:W:EVALS[:extend]"));
        match parts[0] {
            "pimc" => Ok(Player::Pimc(search::Config { worlds: n(1)?, ..search::Config::simulations(n(2)?) })),
            "is" => Ok(Player::Is(issearch::Config { worlds: n(1)?, evaluations: n(2)?, extend: parts[3..].contains(&"extend"), hands: parts[3..].iter().find_map(|p| p.strip_prefix("hands")?.parse().ok()).unwrap_or(4), ..issearch::Config::default() })),
            _ => Err(format!("no player {spec}: pimc:W:SIMS or is:W:EVALS[:extend]")),
        }
    }

    /// The player's answer at the request `game` waits on, and what its search reported.
    fn answer(&self, game: &Game, knowledge: &Knowledge, evaluator: &dyn Evaluator, seed: u64) -> Result<(Answer, serde_json::Value), String> {
        let request = game.pending().cloned().ok_or("the game asks nothing")?;
        let started = Instant::now();
        let guard = CancelGuard::default();
        match self {
            Player::Pimc(c) => {
                let c = search::Config { seed, ..c.clone() };
                let d = search::search(game, &request, knowledge, InfoMode::Decklist, table(), evaluator, &c, &Prior::new(), &guard, &WallClock).map_err(|e| e.0)?;
                let s = d.search.as_ref();
                Ok((d.answer.clone(), json!({
                    "seconds": started.elapsed().as_secs_f64(),
                    "evaluations": d.evaluations,
                    "root_value": d.value,
                    "fusion": s.and_then(|s| s.fusion).map(|f| json!({"worlds": f.worlds, "distinct": f.distinct, "plurality": f.plurality})),
                })))
            }
            Player::Is(c) => {
                let c = issearch::Config { seed, ..c.clone() };
                let (d, s) = issearch::search(game, &request, knowledge, table(), evaluator, &c, &guard).map_err(|e| e.0)?;
                Ok((d.answer.clone(), json!({
                    "seconds": started.elapsed().as_secs_f64(),
                    "evaluations": d.evaluations,
                    "root_value": s.root_value,
                    "net_value": s.net_value,
                    "nodes": s.nodes,
                    "infosets": s.infosets,
                    "worlds": s.worlds,
                    "own_worlds": s.own_worlds,
                    "iterations": s.iterations,
                    "boundary": s.boundary,
                    "hot": s.hot,
                    "capped": s.capped,
                    "deepest_turns": s.deepest_turns,
                    "line_turns": s.line_turns,
                    "line_decisions": s.line_decisions,
                    "root": s.root.iter().map(|(t, p)| json!([format!("{t:?}"), p])).collect::<Vec<_>>(),
                    "range": s.range.iter().map(|(w, own, v, hand)| json!({"weight": w, "own": own, "value": v, "hand": hand})).collect::<Vec<_>>(),
                })))
            }
        }
    }
}

/// A position to value: the trace (by its game's seed) and the answers given before it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub id: u64,
    pub seed: u64,
    pub at: usize,
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A candidate: the engine's answer, the first choice's option it takes (its index among the step's options, and its
/// share of the network's prior there), and whether it is the answer the game's player gave.
struct Candidate {
    answer: Answer,
    first: Option<usize>,
    prior: f32,
    given: bool,
    best: bool,
}

/// The network's value of a game for `seat`: its value of the request waiting, for the seat asked, turned to `seat`'s.
fn value_at(game: &Game, ks: &mut [Knowledge], seat: PlayerId, evaluator: &dyn Evaluator) -> Option<f32> {
    let request = game.pending()?.clone();
    let p = request.player.index();
    ks[p].update(game);
    let (obs, index) = observe_indexed(game, &ks[p], request.player, InfoMode::Decklist)?;
    let answering = Answering::new(game, &request, &obs, &index);
    let m = Machine::new(&answering.ask, &[]);
    let step = m.step()?;
    let e = encode::encode(&obs, step, m.chosen(), &m.names(), table());
    let v = evaluator.evaluate(&FeatureBatch::new(&[&e])).ok()?.value[0].clamp(-1.0, 1.0);
    Some(if request.player == seat { v } else { -v })
}

/// Plays the line on from where `game` stands, both seats the network's alone on their own Decklist observations, and
/// returns its value for `seat` and how it ended ("won", "lost", "draw", "cap", or "stuck" where neither the network nor
/// its fallback answered).
fn continue_line(game: &mut Game, mut ks: Vec<Knowledge>, seat: PlayerId, evaluator: &dyn Evaluator, options: &Options) -> (f32, &'static str) {
    let cap = game.state.turn.number + options.turns;
    let mut payments = vec![Payment::default(); ks.len()];
    for _ in 0..options.max_answers {
        if game.is_over() {
            return match game.state.outcome {
                Outcome::Won(w) if w == seat => (1.0, "won"),
                Outcome::Won(_) => (-1.0, "lost"),
                _ => (0.0, "draw"),
            };
        }
        if game.state.turn.number > cap {
            break;
        }
        let Some(request) = game.pending().cloned() else {
            break;
        };
        let p = request.player.index();
        ks[p].update(game);
        let decided = payments[p].answer_by(game, &request, &ks[p], InfoMode::Decklist, table(), evaluator, Choice::Best, None);
        let answer = match decided {
            Ok(d) => d.answer,
            Err(_) => return (f32::NAN, "stuck"),
        };
        // Noted before it is given, while the objects it names stand; taken back if the engine refuses it.
        let board = cantrip_ai::cancel::board_hash(&game.state);
        let before = payments[p].clone();
        payments[p].played(game, board, &request, &answer);
        if game.answer(request.id, request.player, answer.clone()).is_err() {
            payments[p] = before;
            let filtered = payments[p].filtered(game, &request);
            let Ok(d) = net::answer(game, &filtered, &ks[p], InfoMode::Decklist, table(), evaluator, Choice::Best) else {
                return (f32::NAN, "stuck");
            };
            payments[p].played(game, board, &request, &d.answer);
            if game.answer(request.id, request.player, d.answer.clone()).is_err() {
                return (f32::NAN, "stuck");
            }
        }
    }
    match value_at(game, &mut ks, seat, evaluator) {
        Some(v) => (v, "cap"),
        None => (f32::NAN, "stuck"),
    }
}

/// The candidates at the request `game` waits on (see the module), the network's root value for the seat, the root's
/// encoded first choice (compact), its legal options, and the seat's observation (what the categories are read from).
#[allow(clippy::type_complexity)]
fn candidates(game: &Game, knowledge: &Knowledge, given: &Answer, evaluator: &dyn Evaluator, top: usize) -> Result<(Vec<Candidate>, f32, Vec<u8>, usize, Observation), String> {
    let request = game.pending().cloned().ok_or("the game asks nothing")?;
    let (obs, index) = observe_indexed(game, knowledge, request.player, InfoMode::Decklist).ok_or("no observation of the request")?;
    let answering = Answering::new(game, &request, &obs, &index);
    let eval = |m: &Machine| -> Result<(Vec<f32>, f32, Vec<u8>), String> {
        let step = m.step().ok_or("no step")?;
        let e = encode::encode(&obs, step, m.chosen(), &m.names(), table());
        let out = evaluator.evaluate(&FeatureBatch::new(&[&e])).map_err(|e| e.0)?;
        Ok((out.policy[0].clone(), out.value[0], to_compact(&e)))
    };
    // The first choice: past every step with one legal option.
    let mut root = Machine::new(&answering.ask, &[]);
    while let Some(step) = root.step() {
        let legal: Vec<usize> = step.legal().collect();
        match legal.len() {
            0 => return Err("a step with no legal option".into()),
            1 => root.choose(legal[0]),
            _ => break,
        }
    }
    let step = root.step().ok_or("the answer has no choice")?.clone();
    let legal: Vec<usize> = step.legal().collect();
    let (logits, value, compact) = eval(&root)?;
    let logit = |i: usize| logits.get(i).copied().unwrap_or(MASKED);
    let top_logit = legal.iter().map(|&i| logit(i)).fold(f32::MIN, f32::max);
    let weights: Vec<f32> = legal.iter().map(|&i| (logit(i) - top_logit).exp()).collect();
    let sum: f32 = weights.iter().sum::<f32>().max(1e-30);
    let prior = |i: usize| legal.iter().position(|&l| l == i).map_or(0.0, |k| weights[k] / sum);
    let mut order: Vec<usize> = legal.clone();
    order.sort_by(|&a, &b| logit(b).total_cmp(&logit(a)).then(a.cmp(&b)));

    let mut out: Vec<Candidate> = Vec::new();
    // Each of the likeliest first options, the rest of the answer the network's best.
    for (rank, &first) in order.iter().take(top.max(1)).enumerate() {
        let mut m = root.clone();
        m.choose(first);
        while let Some(s) = m.step() {
            let open: Vec<usize> = s.legal().collect();
            let i = match open.len() {
                0 => break,
                1 => open[0],
                _ => {
                    let (l, _, _) = eval(&m)?;
                    *open.iter().max_by(|&&a, &&b| l.get(a).copied().unwrap_or(MASKED).total_cmp(&l.get(b).copied().unwrap_or(MASKED)).then(b.cmp(&a))).expect("open")
                }
            };
            m.choose(i);
        }
        let Some(factored) = m.answer().cloned() else { continue };
        let Ok(answer) = answering.engine(game, &request, &factored) else { continue };
        if let Some(c) = out.iter_mut().find(|c| c.answer == answer) {
            c.best |= rank == 0;
            continue;
        }
        out.push(Candidate { answer, first: Some(first), prior: prior(first), given: false, best: rank == 0 });
    }
    // The answer given, and its first option where the factoring names it.
    match out.iter_mut().find(|c| c.answer == *given) {
        Some(c) => c.given = true,
        None => {
            let first = answering.factored(&request, given, None).and_then(|f| answering.decompose(&[], &f).ok()).and_then(|tokens| {
                let mut m = Machine::new(&answering.ask, &[]);
                for t in tokens {
                    let s = m.step()?;
                    if s.legal().count() > 1 {
                        return (0..s.tokens.len()).find(|&i| s.tokens[i] == t && s.mask[i]);
                    }
                    if !m.choose_token(t) {
                        return None;
                    }
                }
                None
            });
            out.push(Candidate { answer: given.clone(), first, prior: first.map_or(f32::NAN, prior), given: true, best: false });
        }
    }
    Ok((out, value, compact, legal.len(), obs))
}

/// The reference at one position: a JSON object (see `run`).
/// `before`, the same position valued by an earlier run with the same worlds (`Options::seed`, `worlds`): its
/// candidates' values are kept, and only answers it did not value are played.
pub fn value_position(trace: &Trace, position: &Position, evaluator: &dyn Evaluator, options: &Options, before: Option<&serde_json::Value>) -> Result<serde_json::Value, String> {
    let started = Instant::now();
    let (game, mut seats) = trace.replay(position.at)?;
    let request = game.pending().cloned().ok_or("the replayed game asks nothing")?;
    let seat = request.player;
    let step = trace.steps.get(position.at).ok_or("no answer at the position")?;
    let given = seats.answer(&game, &request, &step.answer)?;
    let knowledge = Knowledge::of(&game, seat);
    let (mut cands, net_value, compact, legal, observation) = candidates(&game, &knowledge, &given, evaluator, options.top)?;
    // The scored players' answers, each a candidate.
    let mut scored = serde_json::Map::new();
    for (name, player) in &options.players {
        let seed = splitmix(options.seed ^ splitmix(position.id ^ 0x5c0e));
        match player.answer(&game, &knowledge, evaluator, seed) {
            Ok((answer, report)) => {
                let c = match cands.iter().position(|c| c.answer == answer) {
                    Some(c) => c,
                    None => {
                        cands.push(Candidate { answer, first: None, prior: f32::NAN, given: false, best: false });
                        cands.len() - 1
                    }
                };
                scored.insert(name.clone(), json!({ "candidate": c, "report": report }));
            }
            Err(e) => {
                scored.insert(name.clone(), json!({ "error": e }));
            }
        }
    }
    // What an earlier run valued, by answer.
    let mut earlier: std::collections::HashMap<String, (Vec<f32>, Vec<String>)> = std::collections::HashMap::new();
    for c in before.and_then(|b| b["candidates"].as_array()).into_iter().flatten() {
        let values: Vec<f32> = c["values"].as_array().map(|v| v.iter().map(|x| x.as_f64().map_or(f32::NAN, |x| x as f32)).collect()).unwrap_or_default();
        let endings: Vec<String> = c["endings"].as_array().map(|v| v.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();
        if values.len() == options.worlds {
            earlier.insert(c["answer"].as_str().unwrap_or("").to_string(), (values, endings));
        }
    }
    let kept: Vec<bool> = cands.iter().map(|c| earlier.contains_key(&format!("{:?}", c.answer))).collect();
    let players = game.state.players.len();
    let prior = Prior::new();
    let mut values = vec![Vec::with_capacity(options.worlds); cands.len()];
    let mut endings: Vec<Vec<String>> = vec![Vec::with_capacity(options.worlds); cands.len()];
    for (c, cand) in cands.iter().enumerate() {
        if let Some((v, e)) = earlier.get(&format!("{:?}", cand.answer)) {
            values[c] = v.clone();
            endings[c] = e.clone();
        }
    }
    let mut worlds = Vec::with_capacity(options.worlds);
    let base = splitmix(options.seed ^ splitmix(position.id.wrapping_add(1)));
    for w in 0..options.worlds {
        let seed = splitmix(base ^ splitmix(w as u64 + 1));
        let mut g = game.fork_for_search();
        g.reseed(seed);
        determinize(&mut g, &knowledge, seat, InfoMode::Decklist, &prior, &mut Rng::new(seed));
        let root = g.checkpoint();
        let opponent_hand: Vec<String> = g.state.seats().filter(|&p| p != seat).flat_map(|p| g.state.zone(Zone::hand(p)).to_vec()).map(|id| g.name(id).to_string()).collect();
        worlds.push(json!({ "opponent_hand": opponent_hand }));
        let ks: Vec<Knowledge> = (0..players).map(|p| if p == seat.index() { knowledge.clone() } else { Knowledge::of(&g, PlayerId(p as u8)) }).collect();
        for (c, cand) in cands.iter().enumerate() {
            if kept[c] {
                continue;
            }
            g.goto(root);
            let Some(r) = g.pending().cloned() else {
                values[c].push(f32::NAN);
                endings[c].push("stuck".into());
                continue;
            };
            if r.player != seat || g.answer(r.id, r.player, cand.answer.clone()).is_err() {
                values[c].push(f32::NAN);
                endings[c].push("refused".into());
                continue;
            }
            let (v, how) = continue_line(&mut g, ks.clone(), seat, evaluator, options);
            values[c].push(v);
            endings[c].push(how.into());
        }
    }
    // The seat's view at its last decision of an earlier turn: what the mode-switch category compares with.
    let previous = (0..position.at)
        .rev()
        .find(|&j| trace.steps[j].seat == seat.0 && trace.steps[j].turn < step.turn)
        .and_then(|j| trace.replay(j).ok())
        .and_then(|(g, _)| observe_indexed(&g, &Knowledge::of(&g, seat), seat, InfoMode::Decklist))
        .map(|(obs, _)| obs);
    let candidates: Vec<serde_json::Value> = cands
        .iter()
        .enumerate()
        .map(|(c, cand)| {
            let live: Vec<f32> = values[c].iter().copied().filter(|v| v.is_finite()).collect();
            let mean = if live.is_empty() { f32::NAN } else { live.iter().sum::<f32>() / live.len() as f32 };
            json!({
                "answer": format!("{:?}", cand.answer),
                "first": cand.first,
                "prior": cand.prior,
                "given": cand.given,
                "best": cand.best,
                "value": mean,
                "values": values[c].iter().map(|v| if v.is_finite() { json!(v) } else { json!(null) }).collect::<Vec<_>>(),
                "endings": endings[c],
            })
        })
        .collect();
    Ok(json!({
        "id": position.id,
        "seed": position.seed,
        "at": position.at,
        "seat": seat.0,
        "turn": game.state.turn.number,
        "kind": cantrip_ai::kind_name(&request.kind),
        "decks": trace.decks,
        "outcome": trace.outcome.get(seat.index()),
        "searched": step.searched,
        "net_value": net_value,
        "legal": legal,
        "candidates": candidates,
        "players": scored,
        "worlds": worlds,
        "position": hex(&compact),
        "observation": observation,
        "previous_observation": previous,
        "seconds": started.elapsed().as_secs_f64(),
    }))
}

/// The luck in a traced game after a position (read by `regret.py`'s label comparison): for each draw after it, the
/// network's value of what was drawn less its expectation over what could have been. The value after a draw is the
/// drawing player's, at the request that follows it when that player is asked there (turned to the position's seat's
/// side); where the other player is asked, nothing that player sees changed, and the draw counts 0. The expectation is
/// over the cards of the drawer's library at the draw, the card drawn and each other one swapped (`rebind_hidden` on a
/// fork, both rows, the multiset of the library kept), as many times as each is there. A request after several draws
/// at once (an opening hand, "draw two") is left uncorrected. Any value function makes the corrected outcome unbiased;
/// a better one takes more of the shuffle out.
///
/// Returns the position's seat, its outcome, the corrections' sum, and how many draws were corrected.
pub fn luck(trace: &Trace, position: &Position, evaluator: &dyn Evaluator) -> Result<serde_json::Value, String> {
    let (mut game, mut seats) = trace.replay(position.at)?;
    let seat = game.pending().ok_or("the replayed game asks nothing")?.player;
    let mut total = 0.0f64;
    let (mut corrected, mut skipped) = (0usize, 0usize);
    let mut terms = Vec::new();
    for (j, step) in trace.steps.iter().enumerate().skip(position.at) {
        let Some(request) = game.pending().cloned() else { break };
        let answer = seats.answer(&game, &request, &step.answer).map_err(|e| format!("answer {j}: {e}"))?;
        let before = game.log.len();
        game.answer(request.id, request.player, answer).map_err(|Rejected(why)| format!("answer {j}: refused: {why}"))?;
        let draws: Vec<(PlayerId, ObjectId)> = game.log[before.min(game.log.len())..].iter().filter_map(|r| match r.event {
            Event::Drew { player, card } => Some((player, card)),
            _ => None,
        }).collect();
        if draws.is_empty() || game.is_over() {
            continue;
        }
        if draws.len() > 1 {
            skipped += draws.len();
            continue;
        }
        let (drawer, drawn) = draws[0];
        let Some(next) = game.pending().cloned() else { continue };
        if next.player != drawer {
            continue;
        }
        // The rows the drawn card was, back into the library.
        let rows_of = |g: &Game, id: ObjectId| {
            let mut chain = vec![id];
            let mut at = id;
            while let Some(p) = (0..g.state.objects.len() as u32).map(ObjectId).find(|&r| g.state.obj(r).became == Some(at)) {
                chain.push(p);
                at = p;
            }
            chain
        };
        let value = |g: &Game| -> Option<f32> {
            let r = g.pending()?.clone();
            let k = Knowledge::of(g, r.player);
            let (obs, index) = observe_indexed(g, &k, r.player, InfoMode::Decklist)?;
            let answering = Answering::new(g, &r, &obs, &index);
            let m = Machine::new(&answering.ask, &[]);
            let e = encode::encode(&obs, m.step()?, m.chosen(), &m.names(), table());
            Some(evaluator.evaluate(&FeatureBatch::new(&[&e])).ok()?.value[0].clamp(-1.0, 1.0))
        };
        let Some(actual) = value(&game) else { continue };
        let library: Vec<ObjectId> = game.state.zone(Zone::library(drawer)).to_vec();
        let card = game.state.obj(drawn).card;
        let mut sum = f64::from(actual);
        let mut n = 1usize;
        let mut ok = true;
        for &y in &library {
            let other = game.state.obj(y).card;
            if other == card {
                sum += f64::from(actual);
                n += 1;
                continue;
            }
            let mut g = game.fork_for_search();
            let (cx, cy) = (rows_of(&game, drawn), rows_of(&game, y));
            g.rebind_hidden(&cx, other);
            g.rebind_hidden(&cy, card);
            g.ask_again();
            match value(&g) {
                Some(v) => {
                    sum += f64::from(v);
                    n += 1;
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            skipped += 1;
            continue;
        }
        let term = (f64::from(actual) - sum / n as f64) * if drawer == seat { 1.0 } else { -1.0 };
        total += term;
        corrected += 1;
        terms.push(json!([j, drawer.0, term]));
    }
    Ok(json!({
        "id": position.id,
        "seat": seat.0,
        "outcome": trace.outcome.get(seat.index()),
        "correction": total,
        "corrected": corrected,
        "skipped": skipped,
        "terms": terms,
    }))
}

/// Every trace in a directory's `*.jsonl` files, by its game's seed (the last line of a seed wins).
pub fn load_traces(dir: &Path) -> Result<std::collections::HashMap<u64, Trace>, String> {
    let mut out = std::collections::HashMap::new();
    let mut files: Vec<_> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "jsonl")).collect();
    files.sort();
    for f in files {
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let t: Trace = serde_json::from_str(line).map_err(|e| format!("{}: {e}", f.display()))?;
            out.insert(t.seed, t);
        }
    }
    Ok(out)
}

/// Values every position on `lanes` lanes, each with its own evaluator from `load`, appending a JSON line a position to
/// `out`; positions whose ids `out` already holds are skipped (a resumed run).
/// `before`: an earlier run's lines by position id, whose valued candidates are kept (`value_position`).
#[allow(clippy::too_many_arguments)]
pub fn run<F>(traces: &std::collections::HashMap<u64, Trace>, positions: &[Position], out: &Path, lanes: usize, options: &Options, before: &std::collections::HashMap<u64, serde_json::Value>, load: F) -> Result<(usize, usize), String>
where
    F: Fn() -> Result<Box<dyn Evaluator>, String> + Send + Sync,
{
    let done: std::collections::HashSet<u64> = std::fs::read_to_string(out)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| v["id"].as_u64())
        .collect();
    let todo: Vec<&Position> = positions.iter().filter(|p| !done.contains(&p.id)).collect();
    let file = std::fs::OpenOptions::new().create(true).append(true).open(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let file = Mutex::new(std::io::BufWriter::new(file));
    let next = AtomicUsize::new(0);
    let (ok, failed) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let started = Instant::now();
    std::thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::new();
        for lane in 0..lanes.max(1) {
            let (todo, file, next, ok, failed, load, before) = (&todo, &file, &next, &ok, &failed, &load, before);
            let h = std::thread::Builder::new().name(format!("regret-{lane}")).stack_size(STACK_SIZE).spawn_scoped(scope, move || -> Result<(), String> {
                let evaluator = load()?;
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(p) = todo.get(i) else { break };
                    let line = match traces.get(&p.seed) {
                        Some(t) => value_position(t, p, evaluator.as_ref(), options, before.get(&p.id)),
                        None => Err(format!("no trace of seed {}", p.seed)),
                    };
                    let line = match line {
                        Ok(v) => {
                            ok.fetch_add(1, Ordering::Relaxed);
                            v
                        }
                        Err(e) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            json!({ "id": p.id, "seed": p.seed, "at": p.at, "error": e })
                        }
                    };
                    let mut f = file.lock().unwrap_or_else(|e| e.into_inner());
                    writeln!(f, "{line}").and_then(|_| f.flush()).map_err(|e| e.to_string())?;
                    let n = ok.load(Ordering::Relaxed) + failed.load(Ordering::Relaxed);
                    if n.is_multiple_of(25) {
                        let s = started.elapsed().as_secs_f64();
                        eprintln!("regret: {n}/{} positions in {s:.0} s ({:.2} a second), {} failed", todo.len(), n as f64 / s.max(1e-9), failed.load(Ordering::Relaxed));
                    }
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
    Ok((ok.into_inner(), failed.into_inner()))
}
