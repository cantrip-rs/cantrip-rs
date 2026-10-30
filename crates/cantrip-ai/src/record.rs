//! The recorder (docs/ai-player-plan.md, "Data"): a finished game made into training samples. The game's line is
//! walked from the deal, as the tree keeps it (`Game::goto`, on a copy: the game played is not touched), and at every
//! answer a seat gave, each sub-step of the answer's decomposition (`cantrip_obs::factor`) is a sample: the answering
//! seat's observation in the recording mode with the step's options, encoded, and the option chosen. The game's end
//! labels them: the result for the seat, the life swing over the turn, whether the seat was dead by the next turn, and
//! the opponents' hands.
//!
//! Paying is no policy step (`pay`): a run of answers to one payment (mana abilities, X raised and lowered, then paid)
//! is one decision, its first request answered as the run ended (announce X as finally announced; pay, or give up).
//! An answer the factoring cannot make is counted by its request kind and skipped (`Failures`), never fatal.

use std::collections::{BTreeMap, HashMap};

use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::encode::{self, to_compact};
use cantrip_obs::factor::{FactoredAnswer, Machine};

use crate::knowledge::front;
use crate::observe::kind_name;
use crate::search::RootStep;
use crate::shard::{GameMeta, GameRecord, Sample, mode_name};
use crate::{Answering, Knowledge, CardIndex, factored_name, final_x, observe_indexed};

/// Answers that made no sample, by "request kind/why".
pub type Failures = BTreeMap<String, u64>;

/// The runtime's card index: the engine's own cards (`CardIndex::engine`).
pub fn table() -> &'static CardIndex {
    CardIndex::engine()
}

/// How the game was dealt, as the table said it.
#[derive(Clone, Debug, Default)]
pub struct Dealt {
    pub seed: u64,
    pub names: Vec<String>,
    /// Each seat's deck name, where the table gave one.
    pub decks: Vec<String>,
    /// The first player was named in the deal, so the host answered who goes first itself: no seat's decision.
    pub auto_start: bool,
    /// A heuristic's value of the position for the seat asked, by the game node the request was asked at, where a
    /// table noted one (the tactical bot's `TacticalModel`, in log-odds of winning: `cantrip-native-host`'s `Note`).
    /// Recorded in the game's meta by decision (`GameMeta::heuristic_values`).
    pub heuristics: HashMap<tree::NodeId, f32>,
}

/// How the game ended: read off the game played, since a concession is not in its tree.
#[derive(Clone, Debug)]
pub struct Ending {
    pub outcome: Outcome,
    pub lost: Vec<bool>,
    /// Closed before it finished (a turn or answer cap): every result is 0.
    pub capped: bool,
}

impl Ending {
    pub fn of(game: &Game, capped: bool) -> Ending {
        Ending {
            outcome: game.state.outcome,
            lost: game.state.players.iter().map(|p| p.lost).collect(),
            capped,
        }
    }
}

/// A run of answers to one payment: the same request kind for the same thing, by the same seat.
pub(crate) fn run_key(request: &Request) -> Option<(PlayerId, u8, ObjectId, ObjectId, bool)> {
    match &request.kind {
        RequestKind::PayCosts {
            source, announcing, ..
        } => Some((request.player, 0, *source, *source, *announcing)),
        RequestKind::UnlessPays {
            source, subject, ..
        } => Some((request.player, 1, *source, *subject, false)),
        _ => None,
    }
}

fn lives(game: &Game) -> Vec<i64> {
    game.state.players.iter().map(|p| p.life as i64).collect()
}

fn swing(lives: &[i64], seat: usize) -> i64 {
    lives[seat]
        - lives
            .iter()
            .enumerate()
            .filter(|&(q, _)| q != seat)
            .map(|(_, l)| l)
            .sum::<i64>()
}

/// What the search made of one request a seat answered by searching: the root's visits per sub-step and its value.
#[derive(Clone, Debug, Default)]
pub struct Root {
    pub visits: Vec<RootStep>,
    pub value: f32,
    /// Whether its visits are a policy target: a fast search of playout-cap randomisation (`selfplay::Options::full_share`)
    /// is not, and its decision is listed in the game's `GameMeta::fast_decisions`; its position is still a value target.
    pub policy: bool,
}

/// The searched requests of a game, by the game node the request was asked at (`game.at` before it was answered).
pub type Roots = HashMap<tree::NodeId, Root>;

/// The samples of a finished game, and the answers that made none. `game` is walked, so it is a copy.
pub fn record(
    game: Game,
    dealt: &Dealt,
    ending: &Ending,
    mode: InfoMode,
    table: &CardIndex,
) -> (GameRecord, Failures) {
    record_searched(game, dealt, ending, mode, table, None)
}

/// A sub-step's visit target: the root's visits over the step's options (by option index, legal and not cut by the
/// cap), normalised to sum to 1; `None` where the search left no visits at this prefix.
fn visit_target(
    root: Option<&Root>,
    chosen: &[cantrip_obs::factor::Token],
    step: &cantrip_obs::factor::Step,
    e: &encode::Encoded,
) -> Option<Vec<(u16, f32)>> {
    let r = root?
        .visits
        .iter()
        .find(|r| r.prefix.as_slice() == chosen)?;
    let mut out: Vec<(u16, f32)> = Vec::new();
    for (t, &n) in r.tokens.iter().zip(&r.visits) {
        if n == 0 {
            continue;
        }
        let Some(i) = (0..step.tokens.len()).find(|&i| step.tokens[i] == *t && step.mask[i]) else {
            continue;
        };
        if e.option_token(i).is_none_or(|at| !e.legal[at]) {
            continue;
        }
        out.push((i as u16, n as f32));
    }
    let total: f32 = out.iter().map(|&(_, n)| n).sum();
    if total <= 0.0 {
        return None;
    }
    for v in &mut out {
        v.1 /= total;
    }
    Some(out)
}

/// `record`, with the search's root visits where the seats searched (`roots`): a sub-step the search visited is
/// labelled with its visit distribution (summing to 1 over the legal options), any other with the option chosen; each
/// searched decision's root value goes in the game's meta.
pub fn record_searched(
    game: Game,
    dealt: &Dealt,
    ending: &Ending,
    mode: InfoMode,
    table: &CardIndex,
    roots: Option<&Roots>,
) -> (GameRecord, Failures) {
    record_distilled(game, dealt, ending, mode, table, roots, None)
}

/// A value distilled at a position the seat is asked: the game there, the seat, its knowledge (`distill::world_mean`).
pub type Distiller<'a> = &'a dyn Fn(&Game, PlayerId, &Knowledge) -> Option<f32>;

/// `record_searched`, and where `distill` is given, its value at each decision recorded in the game's meta
/// (`GameMeta::distill_values`).
pub fn record_distilled(
    mut game: Game,
    dealt: &Dealt,
    ending: &Ending,
    mode: InfoMode,
    table: &CardIndex,
    roots: Option<&Roots>,
    distill: Option<Distiller>,
) -> (GameRecord, Failures) {
    let mut failures = Failures::new();
    let mut fail =
        |kind: &str, why: &str| *failures.entry(format!("{kind}/{why}")).or_default() += 1;
    let end = game.at;
    let seats = game.state.players.len();
    let final_turn = game.state.turn.number;
    let final_lives = lives(&game);
    let nodes = game.tree.path(end);
    let answers: Vec<(_, PlayerId, Answer)> = nodes
        .iter()
        .filter_map(|&n| game.tree.answer(n).map(|(_, p, a)| (n, *p, a.clone())))
        .collect();

    // The request each answer answered, the turn, and the life totals then.
    let mut asked: Vec<Option<Request>> = Vec::with_capacity(answers.len());
    let mut turns: Vec<u32> = Vec::with_capacity(answers.len());
    let mut at_turn_end: HashMap<u32, Vec<i64>> = HashMap::new();
    let mut life_then: Vec<Vec<i64>> = Vec::with_capacity(answers.len());
    for (node, _, _) in &answers {
        let request = game.tree.parent(*node).and_then(|parent| {
            game.goto(parent);
            game.pending().cloned()
        });
        let turn = game.state.turn.number;
        let l = lives(&game);
        if let Some(&last) = turns.last()
            && turn > last
        {
            at_turn_end.entry(last).or_insert_with(|| l.clone());
        }
        asked.push(request);
        turns.push(turn);
        life_then.push(l);
    }
    if let Some(&last) = turns.last() {
        at_turn_end
            .entry(last)
            .or_insert_with(|| final_lives.clone());
    }

    let outcome_for = |seat: PlayerId| -> i8 {
        match ending.outcome {
            _ if ending.capped => 0,
            Outcome::Won(w) if w == seat => 1,
            Outcome::Won(_) => -1,
            _ => 0,
        }
    };
    let mut knowledge: Vec<Knowledge> = (0..seats)
        .map(|i| Knowledge::new(PlayerId(i as u8)))
        .collect();
    let mut samples = Vec::new();
    let mut root_values = Vec::new();
    let mut heuristic_values = Vec::new();
    let mut fast_decisions = Vec::new();
    let mut distill_values: Vec<(u32, f32)> = Vec::new();
    let mut k = 0;
    while k < answers.len() {
        let (node, player, _) = &answers[k];
        let Some(request) = asked[k].clone() else {
            fail("None", "no request");
            k += 1;
            continue;
        };
        // The run of answers this one starts, and its last answer: what the seat finally did.
        let key = run_key(&request);
        let mut last = k;
        while key.is_some()
            && last + 1 < answers.len()
            && asked[last + 1].as_ref().and_then(run_key) == key
        {
            last += 1;
        }
        let start = k;
        k = last + 1;
        let kind = kind_name(&request.kind);
        if request.player != *player {
            fail(&kind, "answered by another seat");
            continue;
        }
        if dealt.auto_start && matches!(request.kind, RequestKind::ChooseStartingPlayer { .. }) {
            continue;
        }
        let Some(parent) = game.tree.parent(*node) else {
            continue;
        };
        let root = roots.and_then(|r| r.get(&parent));
        if let Some(&h) = dealt.heuristics.get(&parent)
            && heuristic_values.last().is_none_or(|&(d, _)| d != start as u32)
        {
            heuristic_values.push((start as u32, h));
        }
        if let Some(r) = root
            && !r.visits.is_empty()
        {
            root_values.push((start as u32, r.value));
            if !r.policy && fast_decisions.last() != Some(&(start as u32)) {
                fast_decisions.push(start as u32);
            }
        }
        game.goto(parent);
        let seat = player.index();
        knowledge[seat].update(&game);
        if let Some(d) = distill
            && distill_values.last().is_none_or(|&(at, _)| at != start as u32)
            && let Some(v) = d(&game, *player, &knowledge[seat])
        {
            distill_values.push((start as u32, v));
        }
        let Some((obs, index)) = observe_indexed(&game, &knowledge[seat], *player, mode) else {
            fail(&kind, "deferred");
            continue;
        };
        let answering = Answering::new(&game, &request, &obs, &index);
        let answer = &answers[last].2;
        let x = match (&request.kind, asked[last].as_ref().map(|r| &r.kind)) {
            (
                RequestKind::PayCosts {
                    announcing: true,
                    x,
                    ..
                },
                _,
            ) => {
                let later: Vec<Answer> =
                    answers[start..].iter().map(|(_, _, a)| a.clone()).collect();
                Some(u16::try_from(final_x(*x, &later)).unwrap_or(u16::MAX))
            }
            (
                RequestKind::UnlessPays { x: Some(_), .. },
                Some(RequestKind::UnlessPays { x: Some(x), .. }),
            ) => Some(*x),
            _ => None,
        };
        let Some(factored) =
            factored_name(&game, answer).or_else(|| answering.factored(&request, answer, x))
        else {
            fail(&kind, "not factored");
            continue;
        };
        let names: Vec<String> = match &factored {
            FactoredAnswer::Name(Some(n)) => vec![n.clone()],
            _ => Vec::new(),
        };
        let tokens = match answering.decompose(&names, &factored) {
            Ok(t) => t,
            Err(_) => {
                fail(&kind, "decompose");
                continue;
            }
        };
        let turn = turns[start];
        let swing_now = |s: usize| swing(&life_then[start], s);
        let life_swing = at_turn_end
            .get(&turn)
            .map_or(0, |end| swing(end, seat) - swing_now(seat)) as f32;
        let dead_next_turn = ending.lost.get(seat).copied().unwrap_or(false)
            && final_turn <= turn + 1
            && !ending.capped;
        let opponent_hand: Vec<u32> = game
            .state
            .seats()
            .filter(|&q| q != *player)
            .flat_map(|q| game.state.zone(Zone::hand(q)).to_vec())
            .map(|id| table.row(&game.cards.get(front(&game, game.state.obj(id).card)).name))
            .collect();
        let mut m = Machine::new(&answering.ask, &names);
        for (j, t) in tokens.into_iter().enumerate() {
            let Some(step) = m.step() else {
                fail(&kind, "more tokens than steps");
                break;
            };
            let Some(i) = (0..step.tokens.len()).find(|&i| step.tokens[i] == t && step.mask[i])
            else {
                fail(&kind, "illegal token");
                break;
            };
            let e = encode::encode(&obs, step, m.chosen(), &m.names(), table);
            if e.option_token(i).is_none_or(|at| !e.legal[at]) {
                fail(&kind, "chosen option cut by the cap");
            } else {
                let visits = visit_target(root, m.chosen(), step, &e)
                    .unwrap_or_else(|| vec![(i as u16, 1.0)]);
                samples.push(Sample {
                    seat: seat as u8,
                    decision: start as u32,
                    step: j as u16,
                    turn: turn.min(u16::MAX as u32) as u16,
                    outcome: outcome_for(*player),
                    life_swing,
                    dead_next_turn,
                    chosen: i as u16,
                    visits,
                    opponent_hand: opponent_hand.clone(),
                    position: to_compact(&e),
                });
            }
            if !m.choose_token(t) {
                fail(&kind, "token refused");
                break;
            }
        }
    }
    game.goto(end);

    let (ended, winner) = match ending.outcome {
        _ if ending.capped => ("cap", None),
        Outcome::Won(w) => ("won", Some(w.index() as u8 + 1)),
        Outcome::Draw => ("draw", None),
        Outcome::InProgress => ("cap", None),
    };
    let meta = GameMeta {
        seed: dealt.seed,
        names: dealt.names.clone(),
        decks: dealt.decks.clone(),
        modes: vec![mode_name(mode).to_string(); seats],
        ended: ended.to_string(),
        winner,
        turns: final_turn,
        answers: answers.len() as u32,
        root_values,
        heuristic_values,
        fast_decisions,
        distill_values,
    };
    (GameRecord { meta, samples }, failures)
}
