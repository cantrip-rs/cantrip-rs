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
use crate::search::{RootOption, RootStep, Searched, StepOrigin, Target};
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
    /// The seats (from 1) the network being trained played, where a game was played against an earlier champion
    /// (self-play's `--opponents`): recorded in the meta (`GameMeta::learners`); empty, every seat.
    pub learners: Vec<u8>,
    /// Where a game began from a scenario's position rather than a deal (self-play's `--starts`): the node of that
    /// position, and the scenario's name. The answers that brought the game there are the script's, no seat's
    /// decisions: they make no samples, and the decisions are counted from the position (as the game's trace is).
    pub start: Option<(tree::NodeId, String)>,
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
    // Allocations and production choices are separate decisions; an option index cannot continue into a changed list.
    if crate::atomic::snow_request(request) { return None; }
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

/// Whether `answer` ends the payment it answers - the cost paid, declined or the casting cancelled - so that a request
/// of the same `run_key` after it is a new ask, not the same payment going on: Thrull Wizard's "unless pays {3}" after
/// its {B} is declined is asked by the same source about the same spell with no answer between them.
pub(crate) fn ends_run(answer: &Answer) -> bool {
    match answer {
        Answer::Cancel => true,
        Answer::Act(a) => matches!(
            a.kind,
            ActionKind::PayCost | ActionKind::DeclineCost | ActionKind::CancelCasting
        ),
        _ => false,
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

/// What the search made of one request a seat answered by searching: the root's visits per sub-step, which sub-steps
/// it reached, its options' values, and the root's value (`Root::of`, from the search's `Searched`).
#[derive(Clone, Debug, Default)]
pub struct Root {
    pub visits: Vec<RootStep>,
    /// Each sub-step of the answer with a choice, and whether the search reached it (`Searched::steps`): a sample is
    /// searched only where the search says so (`Sample::searched`).
    pub steps: Vec<StepOrigin>,
    /// The tokens before the root's first sub-step with a choice, and that step's options as searched, each with its
    /// visits and the values backed up through it over every world (`Searched::root_prefix`, `root_options`): the one
    /// source of the options' root values recorded beside the visits (`Sample::q`).
    pub prefix: Vec<cantrip_obs::factor::Token>,
    pub options: Vec<RootOption>,
    pub value: f32,
    /// Whether its visits are a policy target: a fast search of playout-cap randomisation (`selfplay::Options::full_share`)
    /// is not, and its decision is listed in the game's `GameMeta::fast_decisions`; its position is still a value target.
    pub policy: bool,
    /// Whether its root target is Gumbel's (`Target::Gumbel`): an improved policy at any budget, so a fast search's is a
    /// policy target too where training asks for it (`GameMeta::gumbel_decisions`, train.py's `--fast-policy`).
    pub gumbel: bool,
}

impl Root {
    /// The search's record of a request it answered; `policy` as above.
    pub fn of(searched: &Searched, policy: bool) -> Root {
        Root {
            visits: searched.visits.clone(),
            steps: searched.steps.clone(),
            prefix: searched.root_prefix.clone(),
            options: searched.root_options.clone(),
            value: searched.root_value,
            policy,
            gumbel: searched.target == Target::Gumbel,
        }
    }

    /// Whether the search reached the sub-step after `chosen` and chose its option (`StepOrigin::searched`).
    fn reached(&self, chosen: &[cantrip_obs::factor::Token]) -> bool {
        self.steps.iter().any(|s| s.searched && s.prefix.as_slice() == chosen)
    }

    /// Option `t`'s root value for the seat at the sub-step after `chosen` (`RootOption`'s values over its visits,
    /// every world's, cut edges included): over the root's options whose tokens from its first sub-step run through
    /// it - a whole answer's (`Flat`) at each of its steps, a step's option at that step alone. NaN where none of
    /// them was visited, or where the sub-step lies past what the root's options spell (a later sub-step searched in
    /// steps of its own).
    fn q(&self, chosen: &[cantrip_obs::factor::Token], t: cantrip_obs::factor::Token) -> f32 {
        let Some(rest) = chosen.strip_prefix(self.prefix.as_slice()) else {
            return f32::NAN;
        };
        let (mut n, mut w) = (0u64, 0.0f64);
        for o in &self.options {
            if o.path.len() > rest.len() && o.path[..rest.len()] == *rest && o.path[rest.len()] == t {
                n += u64::from(o.visits);
                w += f64::from(o.value_sum);
            }
        }
        if n > 0 { (w / n as f64) as f32 } else { f32::NAN }
    }
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
/// cap), normalised to sum to 1 (under Gumbel the root's first sub-step holds the Gumbel target scaled to
/// `search::TARGET_SCALE`, made shares the same way), and beside each its root value for the seat (`RootStep::q`;
/// empty where no option's is known); `None` where the search did not reach this sub-step (`Root::reached`) or left no
/// visits on a legal option there. Each option is the search's own by its index (`RootStep::options`), so two options
/// of an equal token keep their own visits; an index that is not this step's option of that token falls back to the
/// first legal option of the token.
fn visit_target(
    root: Option<&Root>,
    chosen: &[cantrip_obs::factor::Token],
    step: &cantrip_obs::factor::Step,
    e: &encode::Encoded,
) -> Option<(Vec<(u16, f32)>, Vec<f32>)> {
    let root = root.filter(|r| r.reached(chosen))?;
    let r = root.visits.iter().find(|r| r.prefix.as_slice() == chosen)?;
    let mut out: Vec<(u16, f32)> = Vec::new();
    let mut q: Vec<f32> = Vec::new();
    for (k, (t, &n)) in r.tokens.iter().zip(&r.visits).enumerate() {
        if n == 0 {
            continue;
        }
        let own = r.options.get(k).copied().filter(|&i| step.tokens.get(i) == Some(t) && step.mask.get(i) == Some(&true));
        let Some(i) = own.or_else(|| (0..step.tokens.len()).find(|&i| step.tokens[i] == *t && step.mask[i])) else {
            continue;
        };
        if e.option_token(i).is_none_or(|at| !e.legal[at]) {
            continue;
        }
        if let Some(entry) = out.iter_mut().find(|(at, _)| usize::from(*at) == i) {
            // The same option twice (the fallback's): its visits once, summed.
            entry.1 += n as f32;
            continue;
        }
        out.push((i as u16, n as f32));
        q.push(r.q.get(k).copied().unwrap_or_else(|| root.q(chosen, *t)));
    }
    let total: f32 = out.iter().map(|&(_, n)| n).sum();
    if total <= 0.0 {
        return None;
    }
    for v in &mut out {
        v.1 /= total;
    }
    if q.iter().all(|x| x.is_nan()) {
        q.clear();
    }
    Some((out, q))
}

/// `record`, with the search's root visits where the seats searched (`roots`): a sub-step the search visited is
/// labelled with its visit distribution (summing to 1 over the legal options) and its options' root values, and marked
/// searched (`Sample::searched`); any other with the option chosen, not searched - a sub-step of a searched decision
/// the search never reached among them; each searched decision's root value goes in the game's meta.
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
    let mut nodes = game.tree.path(end);
    if let Some((at, _)) = &dealt.start
        && let Some(i) = nodes.iter().position(|n| n == at)
    {
        nodes.drain(..=i);
    }
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
    let mut gumbel_decisions = Vec::new();
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
            && !ends_run(&answers[last].2)
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
            if r.gumbel && gumbel_decisions.last() != Some(&(start as u32)) {
                gumbel_decisions.push(start as u32);
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
                Some(final_x(*x, &later))
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
                let (searched, visits, q) = match visit_target(root, m.chosen(), step, &e) {
                    Some((visits, q)) => (true, visits, q),
                    None => (false, vec![(i as u16, 1.0)], Vec::new()),
                };
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
                    searched,
                    q,
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
        gumbel_decisions,
        distill_values,
        learners: dealt.learners.clone(),
        start: dealt.start.as_ref().map(|(_, name)| name.clone()).unwrap_or_default(),
        reanalysed: false,
    };
    (GameRecord { meta, samples }, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cantrip_obs::factor::{Step, Tok, Token};

    fn token(n: u32) -> Token {
        Token::of(Tok::Pick, cantrip_obs::Ref::Object(n), cantrip_obs::Ref::None, 0)
    }

    /// A position whose step's three options are tokens 4 to 6, all legal.
    fn encoded() -> encode::Encoded {
        encode::Encoded {
            bucket: 8,
            len: 7,
            kind: vec![0; 8],
            card: vec![0; 8],
            cats: Vec::new(),
            ptrs: Vec::new(),
            nums: Vec::new(),
            legal: (0..8).map(|i| (4..7).contains(&i)).collect(),
            first_option: 4,
            options: 3,
            cap_hit: false,
            origin: Vec::new(),
        }
    }

    /// Two options of an equal token keep their own visits and values in the target (each by its option index), and the
    /// shares sum to 1; a step recorded with no indices reads as before, its options found by token.
    #[test]
    fn options_of_an_equal_token_keep_their_own_visits() {
        let (a, b) = (token(1), token(2));
        let step = Step { tokens: vec![a, a, b], mask: vec![true; 3] };
        let rs = RootStep { prefix: Vec::new(), tokens: vec![a, a, b], options: vec![0, 1, 2], visits: vec![3, 1, 4], q: vec![0.5, -0.5, 0.25], chosen: b };
        let root = Root {
            visits: vec![rs.clone()],
            steps: vec![StepOrigin { prefix: Vec::new(), chosen: b, searched: true }],
            prefix: Vec::new(),
            options: Vec::new(),
            value: 0.0,
            policy: true,
            gumbel: false,
        };
        let (target, q) = visit_target(Some(&root), &[], &step, &encoded()).expect("a target");
        assert_eq!(target, vec![(0, 3.0 / 8.0), (1, 1.0 / 8.0), (2, 4.0 / 8.0)]);
        assert!((target.iter().map(|t| t.1).sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(q, vec![0.5, -0.5, 0.25]);
        // Without indices (the fallback), an equal token's visits go to its first option, once and summed.
        let old = Root { visits: vec![RootStep { options: Vec::new(), q: Vec::new(), ..rs }], ..root };
        let (target, _) = visit_target(Some(&old), &[], &step, &encoded()).expect("a target");
        assert_eq!(target, vec![(0, 4.0 / 8.0), (2, 4.0 / 8.0)]);
    }
}
