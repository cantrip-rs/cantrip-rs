//! Traces: a game kept as what deals and answers it again, so that any of its positions can be stood in later (a
//! regret measure's sampled positions, re-scored by every later champion). A trace is the deal (the decklists, the seed,
//! who went first) and each answer in order, by the seat that gave it, as a factored answer in Full mode's canonical
//! terms (`cantrip_obs::factor`): the engine's ids are not kept, only what the factoring names, which the same deal and
//! the same answers before it name again.
//!
//! An answer is kept only once it is shown to make the answer given again: the factoring of the engine's answer, or the
//! factored answer that made it (a payment run's first, an X announced a step at a time), or a pass or a cancel, each
//! turned back into the engine's and compared. An action the factoring has no form for - a mana ability a player
//! activated by hand to pay a cost, as the AI's payment does when its plan leaves sources untapped that the plain
//! autotapper would tap - is kept as the request's option it activates and the action's whole words (`ByHand`), which
//! make it again exactly. An answer none of them makes ends the trace there (`Trace::complete` false); what it holds up
//! to there still replays. A step is answered again with `Seats::step`.

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
    /// Where the game began from a scenario's position rather than a deal (self-play's `--starts`): the scenario's name.
    /// Its steps are the answers from that position on; `replay_on` stands them in on the scenario's game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// How the game was dealt beyond its lists and seed, where it was not self-play's plain deal (a table's: the native
    /// host's `--ai-record-traces`): replayed with `deal_with` and these. None: `deal(lists, seed, shuffled, nobody
    /// chooses)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deal: Option<DealSpec>,
}

/// A deal's setup beyond the decklists and the seed (`cantrip_engine::Outside`, the shuffle and who chooses the first
/// player), by card name, as a table dealt it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DealSpec {
    #[serde(default)]
    pub sideboards: Vec<Vec<String>>,
    #[serde(default)]
    pub companions: Vec<Option<String>>,
    #[serde(default)]
    pub basic_lands_outside: Vec<String>,
    #[serde(default)]
    pub minimum_deck_size: u32,
    #[serde(default)]
    pub commander: bool,
    #[serde(default)]
    pub commanders: Vec<Vec<String>>,
    pub shuffle: bool,
    /// The seat (from 0) that chooses who goes first, where one does.
    #[serde(default)]
    pub opening_chooser: Option<u8>,
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
    /// An action the factoring has no form for, kept as itself (`ByHand`); `answer` is then no answer of the step's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_hand: Option<ByHand>,
}

/// An action the factoring has no form for (a mana ability activated by hand to pay a cost): the request's option it
/// is (`RequestKind`'s `options`, by place), and the action's words (its `Debug`), which pick it out exactly among the
/// actions the option can be (`by_hand_actions`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByHand {
    pub option: u32,
    pub action: String,
}

/// The options a request offers to act on, where it has any.
fn options_of(kind: &RequestKind) -> &[ActionOption] {
    match kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } => options,
        _ => &[],
    }
}

/// The actions option `o` can be taken as: as offered, and with each colour or kind of mana named (a source that makes
/// mana of a choice).
fn by_hand_actions(o: &ActionOption) -> impl Iterator<Item = Action> + '_ {
    let base = Action::of(o);
    let colors = std::iter::once(None).chain(Color::ALL.into_iter().map(Some));
    colors.flat_map(move |color| {
        let base = base.clone();
        std::iter::once(base.mana).chain(std::iter::once(None)).chain(Mana::ALL.into_iter().map(Some)).map(move |mana| Action { color, mana, ..base.clone() })
    })
}

impl ByHand {
    /// `given`, an action answering `request`, as an option of the request and its words; none where it is no action, or
    /// no option makes it.
    pub fn of(request: &Request, given: &Answer) -> Option<ByHand> {
        let Answer::Act(a) = given else { return None };
        let options = options_of(&request.kind);
        let k = options.iter().position(|o| by_hand_actions(o).any(|c| c == *a))?;
        Some(ByHand { option: k as u32, action: format!("{a:?}") })
    }

    /// The engine's answer this is to `request`.
    pub fn answer(&self, request: &Request) -> Result<Answer, String> {
        let o = options_of(&request.kind).get(self.option as usize).ok_or("no such option of the request")?;
        by_hand_actions(o).find(|c| format!("{c:?}") == self.action).map(Answer::Act).ok_or_else(|| format!("option {} makes no {}", self.option, self.action))
    }
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

    /// The engine's answer a trace's step makes to `request`: its factored answer's, or the action it keeps by hand.
    pub fn step(&mut self, game: &Game, request: &Request, step: &Step) -> Result<Answer, String> {
        match &step.by_hand {
            Some(h) => {
                // Kept up to date as `answer` keeps it.
                self.answering(game, request).ok_or("no observation of the request")?;
                h.answer(request)
            }
            None => self.answer(game, request, &step.answer),
        }
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
        Trace { seed, decks, lists, steps: Vec::new(), complete: true, outcome: Vec::new(), start: None, deal: None }
    }

    /// Keeps `given`, the answer just about to be given to `request` (see the module), or ends the trace.
    pub fn keep(&mut self, seats: &mut Seats, game: &Game, request: &Request, given: &Answer, made: Option<&FactoredAnswer>, searched: Option<(f32, f32)>) {
        if !self.complete {
            return;
        }
        let step = |answer, by_hand| Step { seat: request.player.0, turn: game.state.turn.number, kind: kind_name(&request.kind), answer, searched, by_hand };
        match seats.factor(game, request, given, made) {
            Some(answer) => self.steps.push(step(answer, None)),
            None => match ByHand::of(request, given).filter(|h| h.answer(request).is_ok_and(|a| a == *given)) {
                Some(h) => {
                    let _ = seats.answering(game, request);
                    self.steps.push(step(FactoredAnswer::AutoPay, Some(h)));
                }
                None => self.complete = false,
            },
        }
    }

    /// The game dealt again, its first `upto` answers given, and each seat's knowledge of it. A trace from a scenario's
    /// position (`start`) is not dealt: `replay_on` takes the scenario's game.
    pub fn replay(&self, upto: usize) -> Result<(Game, Seats), String> {
        self.replay_on(self.dealt()?, upto)
    }

    /// The game dealt again, no answer given: as `deal` says, or self-play's plain deal. A trace from a scenario's
    /// position is not dealt (`replay_on`).
    pub fn dealt(&self) -> Result<Game, String> {
        if let Some(start) = &self.start {
            return Err(format!("the game began at scenario {start}: replay it on the scenario's position"));
        }
        match &self.deal {
            None => deal(&self.lists, self.seed, true, None),
            Some(d) => {
                let outside = Outside {
                    sideboards: &d.sideboards,
                    companions: &d.companions,
                    basic_lands_outside: &d.basic_lands_outside,
                    minimum_deck_size: d.minimum_deck_size,
                    commander: d.commander,
                    commanders: &d.commanders,
                };
                deal_with(&self.lists, &outside, self.seed, d.shuffle, d.opening_chooser.map(PlayerId))
            }
        }
    }

    /// `base` (a deal with no answers) traced from `game`, a game it dealt played on: each answer along the game's line
    /// kept as `keep` keeps it, with no factored answer the player made it from (a table's player made none we know of).
    /// A mana ability a player activated by hand to pay a cost has no factored form unless the autotapper would have
    /// made the same one, so such a payment cuts the trace short there (`complete` false). The result is set once the
    /// game is over. Whether it replays as the game is the caller's to check (`replays_into`).
    pub fn of_game(base: Trace, game: &Game) -> Trace {
        let mut trace = base;
        let end = game.at;
        let mut walk = game.clone();
        let nodes = walk.tree.path(end);
        let first = walk.tree.root();
        walk.goto(first);
        let mut seats = Seats::new(&walk);
        for &n in &nodes {
            let Some((_, player, answer)) = walk.tree.answer(n).map(|(r, p, a)| (*r, *p, a.clone())) else {
                continue;
            };
            let Some(parent) = walk.tree.parent(n) else { continue };
            walk.goto(parent);
            let Some(request) = walk.pending().cloned() else {
                trace.complete = false;
                break;
            };
            if request.player != player {
                trace.complete = false;
                break;
            }
            trace.keep(&mut seats, &walk, &request, &answer, None, None);
            if !trace.complete {
                break;
            }
        }
        if game.is_over() {
            trace.outcome = game
                .state
                .seats()
                .map(|p| match game.state.outcome {
                    Outcome::Won(w) if w == p => 1,
                    Outcome::Won(_) => -1,
                    _ => 0,
                })
                .collect();
        }
        trace
    }

    /// Whether the trace, replayed whole, is `game` up to where it ends: its log the start of the game's - all of it for a
    /// complete trace, unless the game ended otherwise than by an answer (a player conceded, a table ended it), which no
    /// trace holds: then the replay is the game up to there and still going.
    pub fn replays_into(&self, game: &Game) -> bool {
        self.replay(self.steps.len()).is_ok_and(|(g, _)| {
            game.log.starts_with(&g.log) && (!self.complete || g.log == game.log || (game.is_over() && !g.is_over()))
        })
    }

    /// `replay` from `game`, the position the trace's first answer was given at: the deal, or a scenario's position.
    pub fn replay_on(&self, mut game: Game, upto: usize) -> Result<(Game, Seats), String> {
        let mut seats = Seats::new(&game);
        for (i, step) in self.steps.iter().take(upto).enumerate() {
            let request = game.pending().cloned().ok_or_else(|| format!("answer {i}: the game asks nothing"))?;
            if request.player.0 != step.seat {
                return Err(format!("answer {i}: seat {} is asked, not seat {}", request.player.0, step.seat));
            }
            let answer = seats.step(&game, &request, step).map_err(|e| format!("answer {i}: {e}"))?;
            game.answer(request.id, request.player, answer).map_err(|Rejected(why)| format!("answer {i}: refused: {why}"))?;
        }
        Ok((game, seats))
    }
}
