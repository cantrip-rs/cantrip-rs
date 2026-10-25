//! Answering a request with the network alone (docs/ai-player-plan.md, "Answers"): no search. The request's factoring
//! (`cantrip_obs::factor::Machine`) is walked step by step; at each step the seat's observation is encoded with the
//! step's options and evaluated, and the option taken is the policy's best legal one (or one sampled at a
//! temperature). The finished answer is turned into the engine's and checked (`Answering::engine`); payment is no step
//! (`pay`).
//!
//! The factoring leaves some properties of a whole answer to the validator (`Rejection::global`), so a walk can be
//! refused. Then the deepest choice of it that had another option is banned and the walk made again, a bounded number of
//! times; after that the walk that takes the first legal option at every step, banned and retried in the same way. A
//! request that refuses all of them is an error the caller answers some other way.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::encode;
use cantrip_obs::factor::{FactoredAnswer, Machine, Token};

use crate::cancel::{CancelGuard, board_hash};
use crate::eval::{EvalError, Evaluator, FeatureBatch, MASKED};
use crate::{Answering, Knowledge, Rejection, CardIndex, observe_indexed};

/// How an option is taken at a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    /// The legal option the policy rates highest.
    Best,
    /// Sampled from the policy's legal options at this temperature, from this seed.
    Sample { temperature: f32, seed: u64 },
}

/// The network's answer to a request.
#[derive(Clone, Debug)]
pub struct Decision {
    pub answer: Answer,
    pub factored: FactoredAnswer,
    /// The value of the request's first step, for the seat.
    pub value: f32,
    /// The steps evaluated, over every walk.
    pub evaluations: usize,
    /// The walks refused before this one.
    pub rejected: Vec<Rejection>,
    /// Whether the answer is the first-legal walk's, the network's having all been refused.
    pub fallback: bool,
    /// Whether the deadline passed while walking, so that the steps after it took their first legal option.
    pub late: bool,
    /// The steps of the accepted walk (each a choice the factoring asked, evaluated or not).
    pub steps: usize,
    /// The time spent encoding observations, and evaluating them, over every walk.
    pub encode: Duration,
    pub evaluate: Duration,
    /// What the search made of the request, when it was searched (`crate::search`); none for the network alone.
    pub search: Option<crate::search::Searched>,
}

/// A seat's payment in progress (docs/ai-player-plan.md, "Answers"): a run of requests to pay for one thing (announcing
/// X, then paying; or an "unless pays" cost) is one decision, made at its first request and kept to for the rest of
/// the run, as the recorder records it (`record`). The network is never asked the middle of a payment, which it was
/// never shown; asked again there, it would give up a payment half made.
///
/// Around every answer it keeps the AI's cancel guard (`crate::cancel::CancelGuard`): an action the seat gives up is
/// not offered again while the board it was given up on stands.
#[derive(Clone, Debug, Default)]
pub struct Payment {
    run: Option<(PlayerId, u8, ObjectId, ObjectId, bool)>,
    decided: Option<(FactoredAnswer, f32)>,
    guard: CancelGuard,
}

impl Payment {
    /// `answer_by`, except for a request that continues the run of the last one this seat was asked: that is answered
    /// as the run's first was decided (no step evaluated), unless the engine refuses it, when the network is asked.
    #[allow(clippy::too_many_arguments)]
    pub fn answer_by(
        &mut self,
        game: &Game,
        request: &Request,
        knowledge: &Knowledge,
        mode: InfoMode,
        table: &CardIndex,
        evaluator: &dyn Evaluator,
        choice: Choice,
        deadline: Option<Instant>,
    ) -> Result<Decision, EvalError> {
        self.decide(game, request, knowledge, mode, |request, _| {
            crate::net::answer_by(
                game, request, knowledge, mode, table, evaluator, choice, deadline,
            )
        })
    }

    /// The payment's run and the cancel guard around `fresh`, the answer to a request no run decides (the network's
    /// alone, or a search's). `fresh` is given the request with cancelled actions filtered out of it.
    pub fn decide(
        &mut self,
        game: &Game,
        request: &Request,
        knowledge: &Knowledge,
        mode: InfoMode,
        fresh: impl FnOnce(&Request, &CancelGuard) -> Result<Decision, EvalError>,
    ) -> Result<Decision, EvalError> {
        let board = board_hash(&game.state);
        let request = &self.guard.filtered_on(game, board, request);
        let run = crate::record::run_key(request);
        if run.is_some()
            && run == self.run
            && let Some((factored, value)) = self.decided.clone()
            && let Some((obs, index)) = observe_indexed(game, knowledge, request.player, mode)
            && let Ok(answer) =
                Answering::new(game, request, &obs, &index).engine(game, request, &factored)
        {
            self.guard.note(game, board, request, &answer);
            return Ok(Decision {
                answer,
                factored,
                value,
                evaluations: 0,
                rejected: Vec::new(),
                fallback: false,
                late: false,
                steps: 0,
                encode: Duration::ZERO,
                evaluate: Duration::ZERO,
                search: None,
            });
        }
        let mut decided = fresh(request, &self.guard);
        // The filter takes the card's actions out of the request; this keeps a request kind it does not cover from
        // still choosing the same action (as the old guard did at priority).
        if let Ok(d) = &mut decided
            && self.guard.cancelled.iter().find(|(hash, _)| *hash == board).is_some_and(|(_, names)| {
                matches!(&d.answer, Answer::Act(a) if matches!(a.kind, ActionKind::CastSpell | ActionKind::ActivateAbility) && a.source.is_some_and(|source| names.contains(&game.name(source))))
            })
        {
            d.answer = Answer::Pass;
            d.fallback = true;
        }
        self.run = run;
        self.decided = match (&decided, run) {
            (Ok(d), Some(_)) => Some((d.factored.clone(), d.value)),
            _ => None,
        };
        if let Ok(d) = &decided {
            self.guard.note(game, board, request, &d.answer);
        }
        decided
    }

    /// `request` with the actions cancelled on this board filtered out, for a caller answering outside `decide` (a
    /// fallback when a search fails, say) still to respect the guard.
    pub fn filtered(&self, game: &Game, request: &Request) -> Request {
        self.guard
            .filtered_at(game, request, board_hash(&game.state))
    }
}

/// The walks the network makes before falling back.
pub const RETRIES: usize = 8;

/// The seat's answer to `request` (which it must be asked), seen as `mode` lets it see the game through `knowledge`
/// (kept up to date by the caller), its card rows from `table` (the runtime's; the evaluator maps them to its
/// model's).
pub fn answer(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    choice: Choice,
) -> Result<Decision, EvalError> {
    answer_by(
        game, request, knowledge, mode, table, evaluator, choice, None,
    )
}

/// `answer`, by a deadline: once it passes, every step left takes its first legal option without asking the network
/// (`Decision::late`), so the answer comes soon after it.
#[allow(clippy::too_many_arguments)]
pub fn answer_by(
    game: &Game,
    request: &Request,
    knowledge: &Knowledge,
    mode: InfoMode,
    table: &CardIndex,
    evaluator: &dyn Evaluator,
    choice: Choice,
    deadline: Option<Instant>,
) -> Result<Decision, EvalError> {
    let seat = request.player;
    let (obs, index) = observe_indexed(game, knowledge, seat, mode)
        .ok_or_else(|| EvalError("no observation of the request (it is deferred)".into()))?;
    let answering = Answering::new(game, request, &obs, &index);
    let names: Vec<String> = Vec::new();
    let mut random = match choice {
        Choice::Sample { seed, .. } => seed | 1,
        Choice::Best => 1,
    };
    let mut banned: HashSet<(Vec<Token>, Token)> = HashSet::new();
    let mut rejected = Vec::new();
    let mut evaluations = 0;
    let mut value = None;
    let (mut encoding, mut evaluating, mut late) = (Duration::ZERO, Duration::ZERO, false);
    for attempt in 0..2 * RETRIES {
        let network = attempt < RETRIES;
        if attempt == RETRIES {
            banned.clear();
        }
        let mut m = Machine::new(&answering.ask, &names);
        // Each step's chosen token and whether the step had another to take.
        let mut walk: Vec<(Vec<Token>, Token, bool)> = Vec::new();
        while let Some(step) = m.step() {
            let prefix = m.chosen().to_vec();
            let open: Vec<usize> = step
                .legal()
                .filter(|&i| !banned.contains(&(prefix.clone(), step.tokens[i])))
                .collect();
            let open = if open.is_empty() {
                step.legal().collect()
            } else {
                open
            };
            if network && open.len() > 1 && !late && deadline.is_some_and(|d| Instant::now() >= d) {
                late = true;
            }
            let i = if network && open.len() > 1 && !late {
                let started = Instant::now();
                let e = encode::encode(&obs, step, m.chosen(), &m.names(), table);
                let encoded = Instant::now();
                let out = evaluator.evaluate(&FeatureBatch::new(&[&e]))?;
                encoding += encoded - started;
                evaluating += encoded.elapsed();
                evaluations += 1;
                value.get_or_insert(out.value[0]);
                pick(&out.policy[0], &open, choice, &mut random)
            } else {
                open[0]
            };
            walk.push((prefix, step.tokens[i], open.len() > 1));
            m.choose(i);
        }
        let factored = m
            .answer()
            .cloned()
            .ok_or_else(|| EvalError("the machine finished with no answer".into()))?;
        match answering.engine(game, request, &factored) {
            Ok(answer) => {
                if value.is_none() && !late {
                    // A request with one option at every step: its value is still asked, for the caller.
                    if let Some(step) = Machine::new(&answering.ask, &names).step() {
                        let started = Instant::now();
                        let e = encode::encode(&obs, step, &[], &[], table);
                        let encoded = Instant::now();
                        value = Some(evaluator.evaluate(&FeatureBatch::new(&[&e]))?.value[0]);
                        encoding += encoded - started;
                        evaluating += encoded.elapsed();
                        evaluations += 1;
                    }
                }
                let steps = walk.len();
                return Ok(Decision {
                    answer,
                    factored,
                    value: value.unwrap_or(0.0),
                    evaluations,
                    rejected,
                    fallback: !network,
                    late,
                    steps,
                    encode: encoding,
                    evaluate: evaluating,
                    search: None,
                });
            }
            Err(r) => {
                rejected.push(r);
                // Ban the deepest choice that had another option; with none, the walk cannot change.
                match walk.into_iter().rev().find(|w| w.2) {
                    Some((prefix, token, _)) => {
                        banned.insert((prefix, token));
                    }
                    None if network => continue,
                    None => break,
                }
            }
        }
    }
    let last = rejected
        .last()
        .map(|r| format!("{}: {}", r.kind, r.why))
        .unwrap_or_default();
    Err(EvalError(format!(
        "every walk was refused ({} of them; the last: {last})",
        rejected.len()
    )))
}

/// The option of `open` (indices into the step's options) the policy picks.
pub(crate) fn pick(logits: &[f32], open: &[usize], choice: Choice, random: &mut u64) -> usize {
    let logit = |i: usize| logits.get(i).copied().unwrap_or(MASKED);
    match choice {
        Choice::Best => *open
            .iter()
            .max_by(|&&a, &&b| logit(a).total_cmp(&logit(b)).then(b.cmp(&a)))
            .unwrap(),
        Choice::Sample { temperature, .. } => {
            let t = temperature.max(1e-3);
            let top = open.iter().map(|&i| logit(i)).fold(f32::MIN, f32::max);
            let weights: Vec<f64> = open
                .iter()
                .map(|&i| (((logit(i) - top) / t) as f64).exp())
                .collect();
            let total: f64 = weights.iter().sum();
            // xorshift64*
            *random ^= *random >> 12;
            *random ^= *random << 25;
            *random ^= *random >> 27;
            let u = (random.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
            let mut at = u * total;
            for (k, w) in weights.iter().enumerate() {
                if at < *w {
                    return open[k];
                }
                at -= w;
            }
            *open.last().unwrap()
        }
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    const Q: Question = Question::new("Pay", "pay?", "", "Pay", "Decline");

    static A: CardDef = CardDef {
        name: "Guard Test A",
        types: TypeSet::of(&[CardType::Artifact]),
        ..CardDef::BLANK
    };
    static B: CardDef = CardDef {
        name: "Guard Test B",
        types: TypeSet::of(&[CardType::Artifact]),
        ..CardDef::BLANK
    };

    fn game() -> (Game, ObjectId, ObjectId) {
        let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&A, &B])));
        let mut game = Game::new(
            table,
            &Setup {
                decks: vec![Vec::new(), Vec::new()],
                ..Default::default()
            },
        );
        let put = |game: &mut Game, def: &'static CardDef| {
            let id =
                game.state
                    .push_obj(Obj::new(table.id_of(def), PlayerId(0), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id);
            id
        };
        let a = put(&mut game, &A);
        let b = put(&mut game, &B);
        game.state.see_statics();
        (game, a, b)
    }

    fn option(kind: ActionKind, source: ObjectId) -> ActionOption {
        ActionOption {
            source: Some(source),
            ..ActionOption::simple(kind)
        }
    }

    /// A staged cast can be given up at more than one request: a cancel at one board does not forget another board's
    /// cancelled action.
    #[test]
    fn a_cancel_at_one_board_keeps_another() {
        let (game, a, b) = game();
        let board = board_hash(&game.state);
        let other = board.wrapping_add(1);
        let payment = Payment {
            guard: CancelGuard {
                cancelled: vec![(board, vec!["Guard Test A"]), (other, vec!["Guard Test B"])],
                ..CancelGuard::default()
            },
            ..Payment::default()
        };
        let request = Request {
            id: 1,
            player: PlayerId(0),
            kind: RequestKind::Priority {
                options: vec![
                    option(ActionKind::ActivateAbility, a),
                    option(ActionKind::ActivateAbility, b),
                ],
            },
            may_cancel: false,
        };
        let filtered = payment.guard.filtered_at(&game, &request, board);
        let RequestKind::Priority { options } = filtered.kind else {
            panic!("a priority")
        };
        assert_eq!(
            options.iter().filter_map(|o| o.source).collect::<Vec<_>>(),
            vec![b]
        );
    }

    /// A ward's payment given up is declined when the same ward is offered again in the turn, not paid and given up again.
    #[test]
    fn a_declined_ward_is_declined_again() {
        let (mut game, a, b) = game();
        game.state.turn.number = 3;
        let payment = Payment {
            guard: CancelGuard {
                turn_cancelled: Some((3, vec!["Guard Test A"])),
                ..CancelGuard::default()
            },
            ..Payment::default()
        };
        let request = Request {
            id: 1,
            player: PlayerId(0),
            kind: RequestKind::UnlessPays {
                source: a,
                subject: b,
                total: ManaCost::ZERO,
                remaining: ManaCost::ZERO,
                life: 0,
                life_payable: true,
                question: &Q,
                options: vec![
                    option(ActionKind::PayCost, a),
                    option(ActionKind::DeclineCost, a),
                ],
                x: None,
                times: None,
            },
            may_cancel: false,
        };
        let filtered = payment
            .guard
            .filtered_at(&game, &request, board_hash(&game.state));
        let RequestKind::UnlessPays { options, .. } = filtered.kind else {
            panic!("an unless-pays")
        };
        assert_eq!(
            options.iter().map(|o| o.kind).collect::<Vec<_>>(),
            vec![ActionKind::DeclineCost]
        );
    }
}
