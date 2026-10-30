//! Traces: a game kept as what deals and answers it again, so that any of its positions can be stood in later (a
//! regret measure's sampled positions, re-scored by every later champion). A trace is the deal (the decklists, the seed,
//! who went first) and each answer in order, by the seat that gave it, as a factored answer in Full mode's canonical
//! terms (`cantrip_obs::factor`): the engine's ids are not kept, only what the factoring names, which the same deal and
//! the same answers before it name again.
//!
//! An answer is kept only once it is shown to make the answer given again: the factoring of the engine's answer, or the
//! factored answer that made it (a payment run's first, an X announced a step at a time), or a pass or a cancel, each
//! turned back into the engine's and compared. An answer none of them makes ends the trace there (`Trace::complete`
//! false); what it holds up to there still replays.

use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::factor::FactoredAnswer;
use serde::{Deserialize, Serialize};

use crate::{Answering, Knowledge, kind_name, observe_indexed};

/// A game, as what deals and answers it again.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    pub seed: u64,
    /// Each seat's deck name, seat 1 first.
    pub decks: Vec<String>,
    /// Each seat's decklist, by card name, as dealt.
    pub lists: Vec<Vec<String>>,
    pub steps: Vec<Step>,
    /// Whether every answer of the game is kept; a trace cut short ends at the first one that was not.
    pub complete: bool,
    /// The game's result for seat 1 then seat 2: 1, 0 (a draw or a cap) or -1. Empty until the game is over.
    #[serde(default)]
    pub outcome: Vec<i8>,
}

/// One answer, and what the player that gave it knew of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub seat: u8,
    pub turn: u32,
    /// The request's kind (`kind_name`).
    #[serde(default)]
    pub kind: String,
    /// The answer, in Full mode's canonical terms.
    pub answer: FactoredAnswer,
    /// For an answer a search made with a choice: its root value for the seat, and the network's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub searched: Option<(f32, f32)>,
}

/// Each seat's knowledge of a game being traced or replayed, kept up to date as the game goes.
pub struct Seats {
    knowledge: Vec<Knowledge>,
}

impl Seats {
    pub fn new(game: &Game) -> Seats {
        Seats { knowledge: game.state.seats().map(Knowledge::new).collect() }
    }

    fn answering(&mut self, game: &Game, request: &Request) -> Option<Answering> {
        let k = &mut self.knowledge[request.player.index()];
        k.update(game);
        let (obs, index) = observe_indexed(game, k, request.player, InfoMode::Full)?;
        Some(Answering::new(game, request, &obs, &index))
    }

    /// The factored answer that makes `given`, the engine's answer to `request`, again; `made` is the factored answer
    /// the player made it from, where it made one. `None` if none does (see the module).
    pub fn factor(&mut self, game: &Game, request: &Request, given: &Answer, made: Option<&FactoredAnswer>) -> Option<FactoredAnswer> {
        let answering = self.answering(game, request)?;
        let candidates = [answering.factored(request, given, None), made.cloned(), Some(FactoredAnswer::Pass), Some(FactoredAnswer::Cancel)];
        candidates.into_iter().flatten().find(|f| answering.engine(game, request, f).is_ok_and(|a| a == *given))
    }

    /// The engine's answer `step` makes to `request`.
    pub fn answer(&mut self, game: &Game, request: &Request, step: &FactoredAnswer) -> Result<Answer, String> {
        let answering = self.answering(game, request).ok_or("no observation of the request")?;
        answering.engine(game, request, step).map_err(|r| format!("{}: {}", r.kind, r.why))
    }
}

impl Trace {
    /// The deal of a trace, with no answers yet.
    pub fn new(seed: u64, decks: Vec<String>, lists: Vec<Vec<String>>) -> Trace {
        Trace { seed, decks, lists, steps: Vec::new(), complete: true, outcome: Vec::new() }
    }

    /// Keeps `given`, the answer just about to be given to `request` (see the module), or ends the trace.
    pub fn keep(&mut self, seats: &mut Seats, game: &Game, request: &Request, given: &Answer, made: Option<&FactoredAnswer>, searched: Option<(f32, f32)>) {
        if !self.complete {
            return;
        }
        match seats.factor(game, request, given, made) {
            Some(answer) => self.steps.push(Step { seat: request.player.0, turn: game.state.turn.number, kind: kind_name(&request.kind), answer, searched }),
            None => self.complete = false,
        }
    }

    /// The game dealt again, its first `upto` answers given, and each seat's knowledge of it.
    pub fn replay(&self, upto: usize) -> Result<(Game, Seats), String> {
        let mut game = deal(&self.lists, self.seed, true, None)?;
        let mut seats = Seats::new(&game);
        for (i, step) in self.steps.iter().take(upto).enumerate() {
            let request = game.pending().cloned().ok_or_else(|| format!("answer {i}: the game asks nothing"))?;
            if request.player.0 != step.seat {
                return Err(format!("answer {i}: seat {} is asked, not seat {}", request.player.0, step.seat));
            }
            let answer = seats.answer(&game, &request, &step.answer).map_err(|e| format!("answer {i}: {e}"))?;
            game.answer(request.id, request.player, answer).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
        }
        Ok((game, seats))
    }
}
