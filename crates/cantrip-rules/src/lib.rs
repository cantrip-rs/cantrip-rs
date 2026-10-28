//! The core: rules authority and timing (docs/engine-design.md, "Layers: core, shared mechanics, cards").
//!
//! The engine is a step function. [`Game::run`] works the agenda until a frame asks something or the game ends;
//! [`Game::answer`] gives the answer and runs on. Between the two, the game is plain data: copy it with `clone`.

pub mod tree;
mod actions;
#[cfg(test)]
mod as_transforms_tests;
#[cfg(test)]
mod daybound_copy_tests;
#[cfg(test)]
mod token_faces_tests;
#[cfg(test)]
mod attack_effect_tests;
#[cfg(test)]
mod attack_limit_tests;
#[cfg(test)]
mod attack_option_tests;
#[cfg(test)]
mod attacking_tokens_tests;
mod attach;
mod counters;
#[cfg(test)]
mod combat_assignment_tests;
mod day_night;
mod designations;
mod layers;
mod loops;
mod ops;
mod phasing;
#[cfg(test)]
mod any_color_spell_tests;
#[cfg(test)]
mod back_face_targets_tests;
#[cfg(test)]
mod cast_as_spell_tests;
#[cfg(test)]
mod cast_permission_tests;
#[cfg(test)]
mod cloak_tests;
#[cfg(test)]
mod morph_tests;
#[cfg(test)]
mod face_down_reveal_tests;
#[cfg(test)]
mod prototype_tests;
#[cfg(test)]
mod assist_tests;
#[cfg(test)]
mod surge_freerunning_tests;
#[cfg(test)]
mod control_at_once_tests;
#[cfg(test)]
mod can_gain_life_tests;
#[cfg(test)]
mod return_at_once_tests;
#[cfg(test)]
mod linked_text_tests;
#[cfg(test)]
mod loser_check_tests;
#[cfg(test)]
mod history_cut_tests;
#[cfg(test)]
mod name_kind_tests;
#[cfg(test)]
mod inherent_trigger_tests;
#[cfg(test)]
mod copy_card_tests;
#[cfg(test)]
mod copy_spell_tests;
#[cfg(test)]
mod cost_parts_tests;
#[cfg(test)]
mod pay_times_tests;
#[cfg(test)]
mod protection_qualities_tests;
#[cfg(test)]
mod mode_counts_tests;
#[cfg(test)]
mod special_action_cost_tests;
#[cfg(test)]
mod slot_targets_cost_tests;
#[cfg(test)]
mod x_minimum_tests;
#[cfg(test)]
mod together_targets_tests;
#[cfg(test)]
mod suspend_haste_tests;
#[cfg(test)]
mod entering_judged_tests;
#[cfg(test)]
mod enter_rules_tests;
#[cfg(test)]
mod land_play_rules_tests;
#[cfg(test)]
mod instant_sorcery_entering_tests;
#[cfg(test)]
mod attached_entry_tests;
#[cfg(test)]
mod enters_as_tests;
#[cfg(test)]
mod left_battlefield_tests;
#[cfg(test)]
mod land_text_tests;
#[cfg(test)]
mod leave_game_tests;
#[cfg(test)]
mod concession_tests;
#[cfg(test)]
mod count_width_tests;
#[cfg(test)]
mod left_player_choice_tests;
#[cfg(test)]
mod look_back_tests;
#[cfg(test)]
mod functioning_zone_tests;
#[cfg(test)]
mod mixed_zone_tests;
#[cfg(test)]
mod mana_trigger_tests;
#[cfg(test)]
mod mana_replacement_tests;
#[cfg(test)]
mod entry_order_tests;
#[cfg(test)]
mod excess_damage_tests;
#[cfg(test)]
mod must_block_tests;
#[cfg(test)]
mod room_door_tests;
#[cfg(test)]
mod play_permission_tests;
#[cfg(test)]
mod preparation_tests;
#[cfg(test)]
mod convoked_tests;
#[cfg(test)]
mod cycled_tests;
#[cfg(test)]
mod counters_together_tests;
#[cfg(test)]
mod toxic_tests;
#[cfg(test)]
mod remove_up_to_tests;
#[cfg(test)]
mod room_copy_tests;
#[cfg(test)]
mod many_static_sources_tests;
mod replace;
mod resolve;
#[cfg(test)]
mod sacrifice_controller_tests;
mod settle;
mod speed;
mod static_rules;
#[cfg(test)]
mod transformed_cast_tests;
mod turn;
#[cfg(test)]
mod cant_become_untapped_tests;
#[cfg(test)]
mod untap_bans_tests;
#[cfg(test)]
mod untap_others_tests;
mod validate;

use cantrip_api::*;
use cantrip_state::{Frame, FrameKind, Mark, Rng};

pub use actions::{activation_costs, revealed_companion, alternative_cast_total, cast_costs, cast_face_of, cast_total, describe, exile_cast_texts, granted_cast_cost, life_for_cast, granted_cast_total, special_action_costs, sacrifices_for_reduction, replaced_mana_yields, options_with_mana_to_spare, payment_mana_options_with_mana_to_spare};
pub use validate::{check as check_answer, default_name, distribution_problem, name_problem, nameable, selection_verdict, first_choices, attack_targets};
pub use turn::eligible_companions;
pub use layers::{ApplyContract, apply_contract_check, chars, control_order, live, summoning_sick};

/// One game: the state, what has happened, and the card table it is played with.
pub struct Game {
    pub state: GameState,
    /// The event history (`cantrip_api::history`): every event since the game began, in order, with its serial,
    /// turn, step and cause. Cut back only with the journal, when a run is redone.
    pub log: Vec<Record>,
    /// The game log as the search tree, and the node the working state is at (`tree`).
    pub tree: tree::Tree,
    pub at: tree::NodeId,
    pub cards: &'static CardTable,
    /// Check that every rollback restores the state exactly (slow; for tests). See [`Game::verifying`].
    pub verify: bool,
    /// The most events the game records: one that reaches it is a draw (`MAX_GAME_EVENTS`). Smaller in a test.
    pub max_events: usize,
    /// While the triggers of an event are looked for, where the history ends for the triggers' own questions: before the
    /// event, which is already recorded so that the continuous effects that read the history see it (CR 603.10). None
    /// otherwise.
    pub(crate) history_end: Option<usize>,
    /// Where the state triggers stand in the run of a resolution (`StateCheck`). Bookkeeping of a call, not of the game.
    pub(crate) check: StateCheck,
    /// What the offer of the actions a player may take asks of the spells cards would be (`actions::SpellsAsCast`), while
    /// `actions::priority_options` answers. Bookkeeping of a call, not of the game.
    pub(crate) spells_as_cast: std::cell::RefCell<actions::SpellsAsCast>,
}

/// When the state triggers judge a change that records no event (CR 603.8: a state trigger "triggers as soon as the game
/// state matches the condition"): an effect beginning or ending, an object becoming a copy. Outside a resolution, at once.
/// A resolving spell or ability is a run of instructions followed in the order written (CR 608.2c), each a point of the
/// game the state triggers see once it is done, and none of them seen half done - an "each" or "all" is applied to every
/// object at once (CR 611.2c), an exchange of control's two halves together (CR 701.12a). Which calls of a card's code are
/// one instruction only its text says, so a card says it wherever it changes the game with no event more than once:
/// `Game::end_sentence` after an instruction, `Game::one_instruction` around one made of several calls. Under
/// `CANTRIP_VERIFY` a resolution that changes the game with no event twice and says neither between is refused (`unsaid`),
/// so the corpus, which runs every line of every card, finds a card that has not said it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct StateCheck {
    /// How many instructions are open: the resolution itself, then each `one_instruction` (and each engine call that is one)
    /// inside it.
    depth: u8,
    /// The object whose spell or ability is resolving, at depth 1, where one is under way.
    resolving: Option<ObjectId>,
    /// A change with no event that the state triggers have not judged.
    due: bool,
    /// Such a change inside the instruction now open within the resolution (depth 2 and more).
    open: bool,
    /// Such a change by an instruction of the resolution that is done: another, before the state triggers judge it, is a
    /// second instruction the card has not said is one with it, nor apart from it.
    done: bool,
}

impl StateCheck {
    pub(crate) const NONE: StateCheck = StateCheck { depth: 0, resolving: None, due: false, open: false, done: false };
}

/// The most events a game records before it is a draw (`docs/rules-exceptions.md`, RX-1): a run of play that never repeats
/// exactly and that nobody ends - a token more each time round while the other players only pass - is neither a loop the
/// rules can tell nor ever ended by them. 2,097,152 records (about 110 MB of history) is more than a thousand times the
/// longest game the bench plays (about 1,600 events), and some 40,000 rounds of a fifty-event loop played by hand.
pub const MAX_GAME_EVENTS: usize = 1 << 21;

/// With `verify`, every rollback is checked while a game has no more objects than this: twice the most any scenario but
/// a loop played out reaches (about 300).
const VERIFY_EVERY_ONE_UP_TO: usize = 512;
/// Past that, one rollback in this many is checked. A loop played to a draw makes thousands of objects (CR 400.7: every
/// zone change is a new one), and copying them all at every step made a quarter-hour of the test suite out of four
/// scenarios that play in a fraction of a second; one in 32 still checks them thousands of times.
const VERIFY_STRIDE: u32 = 32;

/// A whole copy of a game, its path's journal and the tree with it: one that can go on, or move through the tree,
/// exactly as the original would.
impl Clone for Game {
    fn clone(&self) -> Game {
        Game {
            state: self.state.clone_with_journal(),
            log: self.log.clone(),
            tree: self.tree.clone(),
            at: self.at,
            cards: self.cards,
            verify: self.verify,
            max_events: self.max_events,
            history_end: self.history_end,
            check: StateCheck::NONE,
            spells_as_cast: Default::default(),
        }
    }
}

/// What dealing a game needs.
#[derive(Clone, Debug)]
pub struct Setup {
    /// Each seat's deck, top of the library first.
    pub decks: Vec<Vec<CardId>>,
    /// Each seat's sideboard: the cards it owns outside the game (CR 100.4, 400.11a, 108.3b), which effects that bring a
    /// card into the game from outside it choose among. A seat with no entry has none.
    pub sideboards: Vec<Vec<CardId>>,
    /// In a Limited game, the basic land cards the event supplies: a Limited sideboard holds any number of each (Magic
    /// Tournament Rules; North Wind Avatar's ruling), so each seat always has one of each outside the game. Empty otherwise.
    pub basic_lands_outside: Vec<CardId>,
    /// Each seat's companion, chosen as the deck was built: a card with companion it owns outside the game, dealt into its
    /// sideboard beside the cards there (CR 400.11a). It is revealed before the game if the starting deck fulfils its
    /// condition (CR 103.2b); otherwise it is a sideboard card like any other. A seat with no entry, or None, has none.
    pub companions: Vec<Option<CardId>>,
    /// The minimum deck size of the game's format, as the server that deals it says (CR 100.2a: sixty in Constructed; 100.2b:
    /// forty in Limited): what a companion's condition reads as "the minimum deck size" (Yorion, Sky Nomad).
    pub minimum_deck_size: u32,
    pub seed: u64,
    /// Shuffle the libraries and play the real start of a game (CR 103): who goes first, mulligans. Without it the
    /// libraries stay in the order given, the first seat goes first and both opening hands are dealt and kept, as a
    /// scenario wants.
    pub shuffle: bool,
    /// With `shuffle` off: still ask who goes first and offer mulligans (a scenario's `[Opening]`), this seat choosing.
    pub opening_chooser: Option<PlayerId>,
}

/// A deal with nothing in it, for building on: a Freeform game, with no minimum deck size, every other field empty.
impl Default for Setup {
    fn default() -> Setup {
        Setup {
            decks: Vec::new(),
            sideboards: Vec::new(),
            basic_lands_outside: Vec::new(),
            companions: Vec::new(),
            minimum_deck_size: 0,
            seed: 0,
            shuffle: false,
            opening_chooser: None,
        }
    }
}

/// An answer the engine refused. The game is as it was, the request still outstanding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejected(pub String);

/// The card code stopped: a question it is waiting on, or a cast given up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Suspend,
    /// The player gave up the spell or ability being announced (CR 733.1): everything the frame did is undone.
    Cancel,
}

impl From<Suspend> for Stop {
    fn from(_: Suspend) -> Stop {
        Stop::Suspend
    }
}

pub(crate) type R<T> = Result<T, Stop>;

/// How a frame's run ended.
pub(crate) enum Flow {
    /// Its work is done: take it off the agenda.
    Done,
    /// It stays, to run again from its start with no answers (a priority round, between passes).
    Again,
}

impl Game {
    pub fn new(cards: &'static CardTable, setup: &Setup) -> Game {
        let players = setup.decks.len();
        let mut state = GameState::new(players, setup.seed);
        state.outside_supply = setup.basic_lands_outside.clone();
        state.minimum_deck_size = setup.minimum_deck_size;
        let mut companions: Vec<(PlayerId, ObjectId)> = Vec::new();
        for (seat, deck) in setup.decks.iter().enumerate() {
            let p = PlayerId(seat as u8);
            // A shuffled library is dealt in its shuffled order, so that an object's id says nothing of where its card
            // was in the decklist. The generator draws as shuffling the dealt library would.
            let mut deck = deck.clone();
            if setup.shuffle {
                turn::permute(&mut state, &mut deck);
            }
            let library = deck.iter().map(|&card| (card, Zone::library(p)));
            let sideboard = setup.sideboards.get(seat).into_iter().flatten().chain(&setup.basic_lands_outside).map(|&card| (card, Zone::sideboard(p)));
            for (card, zone) in library.chain(sideboard).collect::<Vec<_>>() {
                let id = state.push_obj(Obj::new(card, p, zone));
                state.zone_push(zone, id);
            }
            if let Some(&Some(card)) = setup.companions.get(seat) {
                let id = state.push_obj(Obj::new(card, p, Zone::sideboard(p)));
                state.zone_push(Zone::sideboard(p), id);
                companions.push((p, id));
            }
        }

        let real = setup.shuffle || setup.opening_chooser.is_some();
        let chooser = match (setup.opening_chooser, setup.shuffle) {
            (Some(p), _) => p,
            (None, true) => PlayerId(state.random(players as u32) as u8),
            (None, false) => PlayerId(0),
        };
        state.update_turn(|t| {
            t.active = chooser;
            t.starting = chooser;
            t.priority = chooser;
        });

        let mut game = Game { state, log: Vec::new(), tree: tree::Tree::new(), at: tree::NodeId::ROOT, cards, verify: false, max_events: MAX_GAME_EVENTS, history_end: None, check: StateCheck::NONE, spells_as_cast: Default::default() };
        // CR 103.2b: the companion chosen as the deck was built is revealed once the starting player is known, if the starting
        // deck - the library as dealt - fulfils its condition; one that doesn't is a sideboard card, and nothing is said of it.
        for (p, id) in companions {
            if turn::eligible_companions(&game, p).contains(&id) {
                game.state.update_player(p, |pl| pl.companion_to_reveal = Some(id));
            }
        }
        if real {
            game.state.frame_push(Frame::new(FrameKind::Opening));
        } else {
            // A scenario's deal: seven each, in turn order, kept.
            for p in game.state.apnap().collect::<Vec<_>>() {
                let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
                for _ in 0..7 {
                    ops::draw(&mut cx, p);
                }
                game.state.update_player(p, |pl| pl.kept = true);
            }
            game.log.clear();
            // CR 103.6a: a card in a hand that may begin the game on the battlefield is asked before the first turn, and CR
            // 103.2b: a companion is revealed.
            let begins = game.state.seats().any(|p| {
                game.state.zone(Zone::hand(p)).iter().any(|&id| cards.get(game.state.obj(id).card).opening_hand.is_some())
                    || game.state.player(p).companion_to_reveal.is_some()
            });
            if begins {
                game.state.frame_push(Frame::new(FrameKind::Opening));
            }
        }
        game.state.journal.entries.clear();
        game.state.journal.base = 0;
        // The journal of the path is the tree's record of its nodes' writes (`tree`): it is never dropped.
        game.state.journal.holds = 1;
        game
    }

    /// The question the game is waiting on, if any.
    pub fn pending(&self) -> Option<&Request> {
        self.state.agenda.last().and_then(|f| f.pending.as_ref())
    }

    pub fn is_over(&self) -> bool {
        self.state.is_over()
    }

    /// Runs until the game asks something or ends.
    pub fn run(&mut self) {
        // What read the history before this run (a view of the game built between answers) is none of the rules' business.
        cantrip_api::history::take_reads();
        loop {
            if self.state.is_over() {
                return;
            }
            if self.state.agenda.is_empty() {
                turn::next_step(self);
                continue;
            }
            let at = self.state.agenda.len() - 1;
            if self.state.agenda[at].pending.is_some() {
                return;
            }
            // RX-1: a game as long as the engine records is a draw, between two frames, so that no frame is left half done.
            if self.log.len() >= self.max_events {
                let mut cx = Cx { g: self, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
                ops::end_game(&mut cx, Outcome::Draw);
                cx.record(Event::GameDrawn { reason: DrawReason::EventLimit });
                return;
            }

            let mark = self.state.mark();
            let log_mark = self.log.len() as u32;
            {
                let frame = &mut self.state.agenda[at];
                frame.mark = mark.0;
                frame.log_mark = log_mark;
            }
            let before = if self.verifying(mark.0) { Some(self.state.clone()) } else { None };

            let kind = self.state.agenda[at].kind;
            let (flow, asked_at) = {
                let mut cx = Cx { g: self, frame: at, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
                let flow = match kind {
                    FrameKind::Opening => turn::opening(&mut cx),
                    FrameKind::BeginStep => turn::begin_step(&mut cx),
                    FrameKind::Priority => turn::priority(&mut cx),
                    FrameKind::Settle => settle::settle(&mut cx),
                    FrameKind::Act { player } => actions::act(&mut cx, player),
                    // A card's effect is a run of instructions, each judged by the state triggers once it is done
                    // (`StateCheck`; CR 603.8, 608.2c, 611.2c), never half-done.
                    FrameKind::Resolve { .. } => {
                        // A staged run goes on with what an earlier stage changed and the state triggers have not judged.
                        let carried = cx.s().agenda[at].unjudged;
                        if carried {
                            cx.st().update_frame(at, |f| f.unjudged = false);
                        }
                        let resolving = cx.resolving().map(|item| item.object);
                        cx.g.check = StateCheck { depth: 1, resolving, due: carried, open: false, done: carried };
                        let flow = resolve::resolve(&mut cx);
                        let check = std::mem::replace(&mut cx.g.check, StateCheck::NONE);
                        if check.due {
                            match flow {
                                Ok(_) => settle::state_triggers_now(&mut cx),
                                // A staged run stopped at its question keeps what it did (`Game::ask_then`): the change is
                                // judged as the run goes on, as if it had never stopped. A run that is redone from its
                                // start when answered makes it again.
                                Err(Stop::Suspend) if cx.s().agenda.get(at).is_some_and(|f| f.staged) => {
                                    cx.st().update_frame(at, |f| f.unjudged = true);
                                }
                                Err(_) => {}
                            }
                        }
                        flow
                    }
                };
                (flow, cx.asked_at)
            };

            loops::after_frame(&mut self.state);
            // A question was asked and the run went on regardless: its work is thrown away and redone when it is answered.
            let flow = match flow {
                Ok(_) if self.state.agenda.get(at).is_some_and(|f| f.pending.is_some()) => {
                    // The run went on past a question it could not stop at (a replacement effect asking which
                    // applies first, or a question of what one does instead): it is redone with the answer, so what it
                    // did after the question is undone now, and the question is asked of the position it was asked in,
                    // as a run that stops at its question leaves it - the cards a replacement effect drew before it
                    // asked which to discard are still there to be chosen.
                    let pending = self.state.agenda[at].pending.clone();
                    let (back_to, log_to) = asked_at.unwrap_or((mark, log_mark as usize));
                    self.state.revert(back_to);
                    self.log.truncate(log_to);
                    self.state.update_frame(at, |f| f.pending = pending);
                    Err(Stop::Suspend)
                }
                // The same, for a run that went on past such a question (the return of a card exiled "until", CR 610.3) and
                // then stopped at another point that asks, which finds the question waiting: it too is put to the position
                // it was asked in, the answer judged there. A run that stopped at its question is there already.
                Err(Stop::Suspend)
                    if let Some((back_to, log_to)) = asked_at
                        && (back_to != self.state.mark() || log_to != self.log.len())
                        && self.state.agenda.get(at).is_some_and(|f| f.pending.is_some() && !f.staged) =>
                {
                    let pending = self.state.agenda[at].pending.clone();
                    self.state.revert(back_to);
                    self.log.truncate(log_to);
                    self.state.update_frame(at, |f| f.pending = pending);
                    Err(Stop::Suspend)
                }
                other => other,
            };
            match flow {
                Ok(Flow::Done) => {
                    self.state.frame_remove(at);
                }
                Ok(Flow::Again) => {
                    let f = &self.state.agenda[at];
                    if !f.answers.is_empty() || !f.conceded.is_empty() {
                        self.state.update_frame(at, |f| {
                            f.answers.clear();
                            f.conceded.clear();
                        });
                    }
                }
                Err(Stop::Suspend) => {
                    if let Some(before) = before {
                        // Rolling back the run must give back exactly the state it started from.
                        let mut probe = self.state.clone_with_journal();
                        probe.rollback(mark);
                        assert!(probe == before, "a rollback did not restore the state: some write was not journalled");
                    }
                    // A staged run that stopped with nothing to ask (a cost a player who left would have paid, CR 800.4f)
                    // goes on at its next stage.
                    if self.state.agenda.get(at).is_some_and(|f| f.staged && f.pending.is_none()) {
                        continue;
                    }
                    return;
                }
                Err(Stop::Cancel) => {
                    // Everything the announcement did is undone (CR 733.1), and the frame goes.
                    self.state.revert(mark);
                    self.log.truncate(log_mark as usize);
                    self.state.frame_remove(at);
                }
            }
            self.forget();
        }
    }

    /// Answers the outstanding request and runs on.
    pub fn answer(&mut self, request: u32, player: PlayerId, answer: Answer) -> Result<(), Rejected> {
        let Some(at) = self.state.agenda.len().checked_sub(1) else {
            return Err(Rejected("nothing is being asked".into()));
        };
        let Some(pending) = self.state.agenda[at].pending.clone() else {
            return Err(Rejected("nothing is being asked".into()));
        };
        if pending.id != request {
            return Err(Rejected(format!("request {request} is not outstanding (request {} is)", pending.id)));
        }
        if pending.player != player {
            return Err(Rejected(format!("request {request} is not for {player:?}")));
        }
        // An answer given here before leads to the node it made: the working state goes there, nothing is replayed.
        if let Some(child) = self.tree.child(self.at, request, player, &answer) {
            self.goto(child);
            return Ok(());
        }
        validate::check(self, &pending, &answer)?;
        let here = self.marks();
        self.tree.close(self.at, here);
        // A run answered without stages is redone from its start, so the events it had recorded are thrown away and
        // the child's begin where the run did, the child keeping them to give back to its parent; the journal only grows (a
        // revert appends the inverse entries).
        let (child_start, displaced) = if self.state.agenda[at].staged {
            (here, &self.log[..0])
        } else {
            let log_mark = self.state.agenda[at].log_mark as usize;
            ((here.0, log_mark), &self.log[log_mark..])
        };
        let child = self.tree.add(self.at, (request, player, answer.clone()), child_start, displaced);

        let frame = &self.state.agenda[at];
        if frame.staged && let Some(waiting) = frame.stand_in.clone() {
            // The controller has chosen who makes the choice a player who left was asked (CR 800.4g): it is asked of them.
            let (next, kind) = *waiting;
            let chosen = match answer {
                Answer::Player(p) => p,
                _ => pending.player,
            };
            loops::asked(&mut self.state, &kind);
            let id = self.state.request_id();
            self.state.update_frame(at, |f| {
                f.stand_in = None;
                f.next_stage = next;
                f.pending = Some(Request { id, player: chosen, kind, may_cancel: false });
            });
        } else if frame.staged {
            let next = frame.next_stage;
            self.state.update_frame(at, |f| {
                f.pending = None;
                f.answers = vec![(Some(player), answer)];
                f.stage = next;
            });
        } else {
            let mark = Mark(frame.mark);
            let log_mark = frame.log_mark as usize;
            let mut answers = frame.answers.clone();
            answers.push((Some(player), answer));
            self.state.revert(mark);
            self.log.truncate(log_mark);
            self.state.update_frame(at, |f| f.answers = answers);
        }
        self.run();
        self.at = child;
        if self.verifying(child.0) {
            // Up to the node the answer was given at and back: the working state must come back exactly. The step
            // takes the history back only to where the child began (`goto` splits it off there) and puts it back, so
            // its length and that tail are what can change; copying the whole history at every answer made a long
            // game quadratic (a 256-round loop to a draw took a minute).
            let from = self.tree.start(child).1;
            let (state, len, tail) = (self.state.clone(), self.log.len(), self.log[from.min(self.log.len())..].to_vec());
            let parent = self.tree.parent(child).expect("a parent");
            self.goto(parent);
            assert_eq!(self.log.len(), here.1, "going back up to the node the answer was given at did not restore its history");
            self.goto(child);
            assert!(
                self.state == state && self.log.len() == len && self.log[from.min(len)..] == tail[..],
                "moving through the game tree did not restore the position"
            );
        }
        Ok(())
    }

    /// Whether this rollback is checked (`verify`): every one in a game of ordinary size, one in `VERIFY_STRIDE` in a
    /// bigger one, picked by `tick` (the journal mark or the node), so a run checks the same ones every time.
    fn verifying(&self, tick: u32) -> bool {
        self.verify && (self.state.objects.len() <= VERIFY_EVERY_ONE_UP_TO || tick % VERIFY_STRIDE == 0)
    }

    /// Where the working state's path ends now: the journal mark and the history's length.
    fn marks(&self) -> (Mark, usize) {
        (self.state.mark(), self.log.len())
    }

    /// Moves the working state to `target` through the tree (`tree`): undoes up to the common ancestor and redoes down
    /// to it. No state is copied.
    pub fn goto(&mut self, target: tree::NodeId) {
        if target == self.at {
            return;
        }
        let here = self.marks();
        self.tree.close(self.at, here);
        let from = self.tree.path(self.at);
        let to = self.tree.path(target);
        let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
        for &node in from[shared..].iter().rev() {
            let (mark, events_from) = self.tree.start(node);
            let (undos, redos) = self.state.step_back(mark);
            let events = self.log.split_off(events_from);
            self.log.extend_from_slice(self.tree.displaced(node));
            self.tree.put_away(node, undos, redos, events);
        }
        for &node in &to[shared..] {
            let (undos, redos, events) = self.tree.take_back(node);
            self.state.step_forward(undos, redos);
            self.log.truncate(self.tree.start(node).1);
            self.log.extend(events);
        }
        self.at = target;
    }

    /// The game ends from outside its rules with `outcome`, whatever is being asked of whom: a draw its players agree to (CR
    /// 104.4a), or a match clock or an abandoned table ending it (`docs/rules-exceptions.md`, RX-4). Its end reveals what
    /// every end does (CR 708.9, `ops::end_game`), and nothing more is asked. Nothing for a game already over.
    pub fn end(&mut self, outcome: Outcome) {
        if self.state.is_over() || outcome == Outcome::InProgress {
            return;
        }
        let mut cx = Cx { g: self, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        ops::end_game(&mut cx, outcome);
    }

    /// `player` concedes (CR 104.3a), whatever is being asked of whom: they leave the game at once (`ops::concede`), after
    /// everything that has already happened. A run waiting on a question keeps the concession at the point it came
    /// (`Frame::conceded`): its replays redo the work done before it exactly and leave the game for the player there, and the
    /// question is asked again if it still is. The conceding player's own announcement is not wound back: what it has done
    /// stands (mana abilities activated, their triggers) and leaving the game takes their objects (CR 800.4a). A run written
    /// in stages is not replayed: the player leaves at once, and a question it was asking them is made by another player,
    /// or not at all for a cost (CR 800.4f-h, `cantrip_state::stand_in`).
    pub fn concede(&mut self, player: PlayerId) {
        if self.state.player(player).lost || self.state.is_over() {
            return;
        }
        let mut now = true;
        let mut staged_at = None;
        if let Some(at) = self.state.agenda.len().checked_sub(1)
            && self.state.agenda[at].pending.is_some()
        {
            let frame = self.state.agenda[at].clone();
            if frame.kind == (FrameKind::Act { player }) {
                self.state.frame_remove(at);
            } else if frame.staged {
                staged_at = Some(at);
            } else {
                // Wound back as an answer winds it, the concession kept where it came: the replay does the rest.
                self.state.revert(Mark(frame.mark));
                self.log.truncate(frame.log_mark as usize);
                let cursor = frame.answers.len() as u32;
                self.state.update_frame(at, |f| {
                    f.pending = None;
                    f.conceded.push((cursor, player));
                });
                now = false;
            }
        }
        if now {
            let mut cx = Cx { g: self, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
            ops::concede(&mut cx, player);
        }
        // A staged run's question to the player who left is made by another, or not at all (CR 800.4f-h).
        if let Some(at) = staged_at
            && let Some(pending) = self.state.agenda[at].pending.clone()
            && self.state.player(pending.player).lost
        {
            match cantrip_state::stand_in(&self.state, &pending.kind, pending.player) {
                cantrip_state::StandIn::Unpaid | cantrip_state::StandIn::NotAsked => {
                    let next = self.state.agenda[at].next_stage;
                    self.state.update_frame(at, |f| {
                        f.pending = None;
                        f.answers = vec![(None, Answer::Pass)];
                        f.stage = next;
                    });
                }
                cantrip_state::StandIn::By(other) => {
                    let id = self.state.request_id();
                    self.state.update_frame(at, |f| f.pending = Some(Request { id, player: other, ..pending }));
                }
                cantrip_state::StandIn::Chosen { controller, source, candidates } => {
                    let id = self.state.request_id();
                    let next = self.state.agenda[at].next_stage;
                    let ask = RequestKind::ChoosePlayer { source, candidates, verb: "makes the choice", amount: 0, subject: None };
                    self.state.update_frame(at, |f| {
                        f.stand_in = Some(Box::new((next, pending.kind)));
                        f.pending = Some(Request { id, player: controller, kind: ask, may_cancel: false });
                    });
                }
            }
        }
        self.forget();
        self.run();
    }

    /// Drops journal entries nothing can roll back to: everything before the oldest run still waiting, when no
    /// checkpoint is held.
    fn forget(&mut self) {
        if self.state.journal.holds > 0 {
            return;
        }
        let oldest = self.state.agenda.iter().filter(|f| f.pending.is_some() || !f.answers.is_empty()).map(|f| f.mark).min();
        let keep = oldest.unwrap_or_else(|| self.state.mark().0);
        self.state.forget_before(Mark(keep));
    }

    /// The node the game is at, for a search to come back to with [`Game::goto`].
    pub fn checkpoint(&self) -> tree::NodeId {
        self.at
    }

    /// A copy of the game for a search to play on: the working state, the journal back only as far as a run still
    /// waiting can be redone from (what `forget` keeps), the history, and a tree of its own rooted here. It answers,
    /// cancels and moves through its tree as the game would, at or after the fork and never before it: its tree's ids
    /// begin past the game's, so `goto` to a node of the true past panics. Costs the state and the journal's tail, not
    /// the game so far. The generator is the game's own; [`Game::reseed`] gives it another.
    pub fn fork_for_search(&self) -> Game {
        let waiting = self.state.agenda.iter().filter(|f| f.pending.is_some() || !f.answers.is_empty()).map(|f| f.mark).min();
        let floor = Mark(waiting.unwrap_or(self.state.mark().0));
        let state = self.state.clone_with_journal_from(floor);
        let root = self.tree.next();
        let tree = tree::Tree::rooted(root, (state.mark(), self.log.len()));
        Game { state, log: self.log.clone(), tree, at: root, cards: self.cards, verify: self.verify, max_events: self.max_events, history_end: None, check: StateCheck::NONE, spells_as_cast: Default::default() }
    }

    /// Gives the game's generator another seed (`GameState::reseed`): a fork for search that must not shuffle or roll
    /// as the real game will.
    pub fn reseed(&mut self, seed: u64) {
        self.state.reseed(seed);
    }

    /// Asks the waiting question again, of the position as it is now: for a fork whose hidden cards were dealt anew
    /// (`rebind_hidden`, `GameState::reorder_library`) after the question was put to the real ones - a priority's
    /// options, the cards a choice is among, and whatever the run did before it asked. The run that asked is rolled
    /// back to where it began and run again, the answers it was given replayed, as an answer would. A question asked
    /// between the stages of a staged frame (`ask_then`) is left as it was asked: its stage cannot be run again. Only
    /// on a fork still at its root.
    pub fn ask_again(&mut self) {
        assert_eq!(self.tree.len(), 1, "asking again in a game whose tree has left its root");
        assert_ne!(self.tree.root(), tree::NodeId::ROOT, "only a game forked for search (`fork_for_search`) is dealt anew");
        let Some(at) = self.state.agenda.len().checked_sub(1) else { return };
        let frame = &self.state.agenda[at];
        if frame.pending.is_none() || frame.staged {
            return;
        }
        let (mark, log_mark) = (Mark(frame.mark), frame.log_mark as usize);
        self.state.revert(mark);
        self.log.truncate(log_mark);
        self.state.update_frame(at, |f| f.pending = None);
        self.run();
    }

    /// Gives a hidden object another card, for a search's world (`cantrip_ai::determinize`): `id` and the rows it
    /// became from (`chain`, `id` first, every one of them hidden from the player the world is for). Only on a fork
    /// still at its root (`fork_for_search`): the rebind is no part of the journal (`GameState::rebind_hidden`), and a
    /// node the fork's tree had left would bring the old cards back. The history's records of those objects' abilities
    /// are rewritten too (a face-down spell's, a hidden card's ability activated from a hand).
    pub fn rebind_hidden(&mut self, chain: &[ObjectId], card: CardId) {
        assert_eq!(self.tree.len(), 1, "rebinding a hidden object of a game whose tree has left its root");
        assert_ne!(self.tree.root(), tree::NodeId::ROOT, "only a game forked for search (`fork_for_search`) is dealt anew");
        let old = self.state.obj(chain[0]).card;
        if old == card {
            return;
        }
        let mut abilities = vec![(old, card)];
        let (old_back, new_back) = (self.cards.face(old, true), self.cards.face(card, true));
        if old_back != old {
            abilities.push((old_back, new_back));
        }
        let statics = [card, new_back].iter().any(|c| self.cards.statics[c.0 as usize]);
        self.state.rebind_hidden(chain, card, &abilities, statics);
        // The rows an object left as it moved while hidden that showed its face (CR 400.7) show the new card's, printed, as
        // a hidden card is. A face-down object's show its face-down values, the world's too.
        let old_faces: Vec<CardId> = abilities.iter().map(|&(from, _)| from).collect();
        let old_names: Vec<&'static str> = old_faces.iter().map(|&c| self.cards.get(c).name).collect();
        let shows = |l: &LastKnown| old_names.contains(&l.name) || l.abilities.iter().flatten().any(|a| old_faces.contains(&a.card));
        let rows: Vec<(ObjectId, LastKnown)> = self
            .state
            .hidden_rows_showing(chain, shows)
            .into_iter()
            .map(|id| {
                let face = self.cards.face(card, self.state.obj(id).transformed);
                let c = layers::chars_as_face(&self.cards, &self.state, &self.log, id, face);
                let mana_value = cantrip_api::mana_value_of(&|c| self.cards.get(c), &self.state, id, &c);
                let row = LastKnown {
                    id,
                    card: c.card,
                    controller: self.state.obj(id).controller,
                    name: c.name,
                    name_set: false,
                    supertypes: c.supertypes,
                    no_cost: false,
                    types: c.types,
                    subtypes: c.subtypes,
                    colors: c.colors,
                    power: c.power,
                    toughness: c.toughness,
                    keywords: c.keywords,
                    toxic: c.toxic,
                    protection: c.protection,
                    protection_colorless: c.protection_colorless,
                    protection_players: c.protection_players,
                    protection_types: c.protection_types,
                    protection_subtypes: c.protection_subtypes,
                    protection_mana_value: c.protection_mana_value,
                    spell: c.spell,
                    protection_colored_spells: c.protection_colored_spells,
                    protection_everything: c.protection_everything,
                    hexproof_from: c.hexproof_from,
                    must_attack: c.must_attack,
                    attacking: false,
                    mana_value,
                    abilities: c.abilities,
                    words: Words::NONE,
                };
                (id, row)
            })
            .collect();
        self.state.rebind_hidden_rows(chain, &rows, shows);
        let patch = |source: ObjectId, ability: &mut AbilityRef| {
            if chain.contains(&source)
                && let Some(&(_, to)) = abilities.iter().find(|(from, _)| *from == ability.card)
            {
                ability.card = to;
            }
        };
        for record in &mut self.log {
            if let Cause::Ability { source, ability } = &mut record.cause {
                patch(*source, ability);
            }
            match &mut record.event {
                Event::AbilityActivated { source, ability, .. }
                | Event::Resolved { source, ability }
                | Event::Triggered { source, ability }
                | Event::ModesChosen { source, ability, .. }
                | Event::Done { source, ability, .. } => patch(*source, ability),
                _ => {}
            }
        }
    }

    /// Back to a node a search left from: [`Game::goto`].
    pub fn rollback(&mut self, to: tree::NodeId) {
        self.goto(to);
    }

    /// An object's characteristics.
    pub fn chars(&self, id: ObjectId) -> Chars {
        layers::chars(self.cards, &self.state, &self.log, id)
    }

    /// The name an object shows (its copiable values' name).
    pub fn name(&self, id: ObjectId) -> &'static str {
        self.chars(id).name
    }

    /// The name of what an object is to one who knows every card, as a script or a log does: its name (`name`), but for a
    /// card exiled face down, which has none (CR 406.3a), its card's. Never what a player is shown.
    pub fn known_name(&self, id: ObjectId) -> &'static str {
        let obj = self.state.obj(id);
        if obj.face_down_card() { self.cards.get(obj.card).name } else { self.name(id) }
    }

    /// Whether `player` can't gain life right now (CR 119.7): an effect or a static ability on the battlefield says so.
    pub fn cant_gain_life(&self, player: PlayerId) -> bool {
        static_rules::cant_gain_life(self, self.cards, player)
    }

    /// Whether a static ability on the battlefield gives `player` the keyword ability `keyword` right now (CR 702.18a: "You
    /// have shroud").
    pub fn player_has(&self, player: PlayerId, keyword: Keyword) -> bool {
        layers::player_has(self.cards, &self.state, &self.log, player, keyword)
    }

    /// Whether a static ability on the battlefield gives `player` protection from `from` right now (CR 702.16k: "You have
    /// protection from each of your opponents").
    pub fn player_protected_by_static_from(&self, player: PlayerId, from: PlayerId) -> bool {
        layers::player_protected_by_static(self.cards, &self.state, &self.log, player, from).is_some()
    }

    /// Whether `player` can't lose life right now (CR 119.8): a static ability on the battlefield says so.
    pub fn cant_lose_life(&self, player: PlayerId) -> bool {
        static_rules::cant_lose_life(self, self.cards, player)
    }

    /// Whether `player` doesn't lose the game for `reason` right now (CR 104.3): a static ability on the battlefield says so.
    pub fn doesnt_lose(&self, player: PlayerId, reason: LossReason) -> bool {
        static_rules::doesnt_lose(self, self.cards, player, reason)
    }

    /// Whether `player` can't win the game right now (CR 104.2b): a static ability on the battlefield says so.
    pub fn cant_win(&self, player: PlayerId) -> bool {
        static_rules::cant_win(self, self.cards, player)
    }

    /// Whether damage dealt to `player` right now is dealt as though its source had infect (CR 702.90b).
    pub fn damage_as_though_infect(&self, player: PlayerId) -> bool {
        static_rules::damage_as_though_infect(self, self.cards, player)
    }

    /// Whether `player` can't get `kind` counters right now (CR 122.1, 101.2).
    pub fn player_counters_banned(&self, player: PlayerId, kind: PlayerCounter) -> bool {
        static_rules::player_counters_banned(self, self.cards, player, kind)
    }

    /// Whether `creature` may attack as though it didn't have defender right now (CR 702.3b, 609.4): an effect or a static
    /// ability on the battlefield says so. Whether it has defender at all is asked of its characteristics.
    pub fn may_attack_despite_defender(&self, creature: ObjectId) -> bool {
        static_rules::may_attack_despite_defender(self, self.cards, creature)
    }

    /// Whether the static abilities on the battlefield let `creature` attack (CR 508.1c): "this creature can't attack unless
    /// ..." (Patchwork Beastie). A resolved "can't attack" effect is asked of the effects.
    pub fn attack_rules_allow(&self, creature: ObjectId) -> bool {
        static_rules::attack_rules_allow(self, self.cards, creature)
    }

    /// The sources of the ordinary mana in `player`'s pool that a rule tells apart from other mana (CR 106.3): mana the player
    /// spends only by choosing it, which the pool shows as that source's.
    pub fn mana_told_apart(&self, player: PlayerId) -> Vec<ObjectId> {
        static_rules::mana_told_apart(self, self.cards, player)
    }

    /// Whether `player` may look at the top card of their library any time (CR 401.2), as a permanent they control says.
    pub fn may_look_at_library_top(&self, player: PlayerId) -> bool {
        actions::may_look_at_library_top(self, self.cards, player)
    }
}

/// The view of the game a frame's run has: what card code is handed as `&mut dyn Game`.
pub(crate) struct Cx<'a> {
    pub g: &'a mut Game,
    /// The frame being run.
    pub frame: usize,
    /// How many of its answers have been taken.
    pub cursor: usize,
    /// The source of the mana ability running (CR 106.3, 605): what produces the mana it adds. None outside one, where the
    /// mana a resolving spell or ability adds is that spell's or the ability's source's.
    pub mana_source: Option<ObjectId>,
    /// A permanent about to enter the battlefield, as it would exist there (CR 614.12): what `Query::chars` and
    /// `Query::lost_all_abilities` answer of it while the effects that modify how it enters are judged
    /// (`ops::judged_entering`).
    pub entering: Option<ops::WouldExist>,
    /// The replacement effects that have had their one opportunity at the event whose modified event is being carried out
    /// (CR 614.5, `replace::run_then`): a card Thought Reflection's "draw two cards instead" draws meets every draw replacement
    /// but that Thought Reflection's. Empty outside one.
    pub lineage: Vec<replace::Applied>,
    /// Where the journal and the history stood when this run's question was asked: a run that goes on past a question it
    /// could not stop at (`ask_core`) is wound back to here, so the question is put to the position it was asked in, with
    /// the objects it names, and not to the one the run began from.
    pub asked_at: Option<(Mark, usize)>,
    /// The player whose replacement effect or mana ability is being carried out (CR 609.1, 605): the player `doer` names,
    /// over the frame's. None outside one.
    pub doer: Option<PlayerId>,
    /// The mana a permanent tapped for mana is producing (CR 106.12b), held while its mana ability's effect runs so that the
    /// replacement effects on the production meet all of it before any is added (`actions::activate_mana`): each addition,
    /// to whichever player's pool (Spectral Searchlight's "that player adds"). None outside such an ability, and while nothing
    /// could replace its mana.
    pub producing: Option<Vec<Produced>>,
}

/// One addition of mana to a pool held as part of a production (`Cx::producing`): the player whose pool it goes to, its type,
/// how much, its restriction (CR 106.6) and whether it lasts until end of combat (CR 106.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Produced {
    pub to: PlayerId,
    pub mana: Mana,
    pub amount: u32,
    pub restriction: Option<ManaRestriction>,
    pub lasting: bool,
}

impl<'a> Cx<'a> {
    pub fn s(&self) -> &GameState {
        &self.g.state
    }

    pub fn st(&mut self) -> &mut GameState {
        &mut self.g.state
    }

    pub fn cards(&self) -> &'static CardTable {
        self.g.cards
    }

    /// A player who conceded at this point of the run leaves the game here, on every replay (`Game::concede`). Again at the
    /// same point does nothing more: a player who has left does not leave again.
    /// How the source of the spell or ability resolving now acquired that ability (CR 607.5), where `source` is that source:
    /// what a delayed or reflexive triggered ability it creates is linked by. Own for anything else.
    pub(crate) fn resolving_acquired(&self, source: ObjectId) -> Acquired {
        let s = &self.g.state;
        let serial = s.agenda.iter().rev().find_map(|f| match f.kind {
            cantrip_state::FrameKind::Resolve { serial } => Some(serial),
            _ => None,
        });
        serial
            .and_then(|serial| s.stack.iter().find(|i| i.serial == serial))
            .filter(|i| i.object == source)
            .map_or(Acquired::OWN, |i| i.acquired)
    }

    pub(crate) fn concessions_here(&mut self) {
        if self.g.state.agenda[self.frame].conceded.is_empty() {
            return;
        }
        let here: Vec<PlayerId> =
            self.g.state.agenda[self.frame].conceded.iter().filter(|&&(at, _)| at as usize == self.cursor).map(|&(_, p)| p).collect();
        for p in here {
            ops::concede(self, p);
        }
    }

    /// A question a rule procedure asks a player about their own game action - priority, attacks, blocks, the discard to hand
    /// size, a mulligan, the order of their triggers, a choice of the rules about their own objects: None when the player has
    /// left the game, whether before or as it is asked (a concession at this point of the run), for the action ends with them
    /// and no one makes it (CR 800.4a, 800.4d, 800.4j). The procedure goes on as the player's leaving requires.
    pub(crate) fn ask_rule(&mut self, player: PlayerId, kind: RequestKind) -> R<Option<Answer>> {
        self.concessions_here();
        if self.g.state.player(player).lost {
            return Ok(None);
        }
        self.ask_core(player, kind, false).map(Some)
    }

    pub fn ask_core(&mut self, player: PlayerId, kind: RequestKind, may_cancel: bool) -> R<Answer> {
        // A real choice, asked or answered on a re-run, ends a run of passes (`loops`; RX-4).
        // A player who conceded at this point of the run leaves the game here, on every replay (`Game::concede`).
        let any_conceded = !self.g.state.agenda[self.frame].conceded.is_empty();
        self.concessions_here();
        // A player who has left the game makes no choice: a cost is not paid, and anything else is chosen by another player
        // (CR 800.4f-h). Any question this asks is one of the run's, replayed as the others are.
        if self.g.state.player(player).lost {
            return match cantrip_state::stand_in(&self.g.state, &kind, player) {
                // No one is asked: a procedure that can tell asks through `ask_rule`.
                cantrip_state::StandIn::Unpaid | cantrip_state::StandIn::NotAsked => Ok(Answer::Pass),
                cantrip_state::StandIn::By(other) => self.ask_core(other, kind, may_cancel),
                cantrip_state::StandIn::Chosen { controller, source, candidates } => {
                    let ask = RequestKind::ChoosePlayer { source, candidates: candidates.clone(), verb: "makes the choice", amount: 0, subject: None };
                    let chosen = match self.ask_core(controller, ask, may_cancel)? {
                        Answer::Player(p) if candidates.contains(&p) => p,
                        _ => candidates[0],
                    };
                    self.ask_core(chosen, kind, may_cancel)
                }
            };
        }
        // The answer given here before, replayed: only ever the answer of the player asked, and after a concession earlier in the
        // run only one that still answers the question (the game may have changed under it).
        let replay = {
            let frame = &self.g.state.agenda[self.frame];
            let changed = any_conceded && frame.conceded.iter().any(|&(at, _)| at as usize <= self.cursor);
            frame
                .answers
                .get(self.cursor)
                .filter(|(by, _)| by.is_none_or(|by| by == player))
                .map(|(_, answer)| answer)
                .filter(|answer| !changed || validate::check(&*self.g, &Request { id: 0, player, kind: kind.clone(), may_cancel }, answer).is_ok())
                .cloned()
        };
        if replay.is_none() && self.cursor < self.g.state.agenda[self.frame].answers.len() {
            let (at, cursor) = (self.frame, self.cursor);
            self.g.state.update_frame(at, |f| {
                f.answers.truncate(cursor);
            });
        }
        loops::asked(&mut self.g.state, &kind);
        if let Some(answer) = replay {
            self.cursor += 1;
            if matches!(
                kind,
                RequestKind::DistributeCards { .. } | RequestKind::SearchLibrary { .. } | RequestKind::ChooseCards { .. } | RequestKind::ChooseName { .. } | RequestKind::ChooseTarget { .. }
            ) {
                self.shown(player, &kind);
            }
            return Ok(answer);
        }
        let frame = &self.g.state.agenda[self.frame];
        // A question is already waiting: an operation that cannot carry the suspension out (a zone change asking
        // which replacement applies first) went on with the first, and the run is abandoned and replayed once the
        // answer is in (see `Game::run`). No second question replaces it.
        if frame.pending.is_some() {
            return Err(Stop::Suspend);
        }
        settle::verify_control(self);
        let id = self.g.state.request_id();
        let at = self.frame;
        self.g.state.update_frame(at, |f| f.pending = Some(Request { id, player, kind, may_cancel }));
        // The position the question is asked in: a run that goes on past it is wound back to here (`Game::run`).
        self.asked_at = Some((self.g.state.mark(), self.g.log.len()));
        Err(Stop::Suspend)
    }

    /// The hidden cards a question showed its player, recorded as it is answered (`Event::Shown`): the cards a scry,
    /// surveil or look at the top looked at, a library searched, a hand chosen from. Only the questions that show cards;
    /// priority and combat name nothing hidden.
    #[inline(never)]
    fn shown(&mut self, player: PlayerId, kind: &RequestKind) {
        let mut hidden: Vec<ObjectId> = Vec::new();
        kind.objects(&mut |id| {
            if id.index() < self.g.state.objects.len() && !hidden.contains(&id) && hidden_from(&self.g.state, player, id) {
                hidden.push(id);
            }
        });
        for card in hidden {
            self.record(Event::Shown { player, card });
        }
    }

    /// A reflexive triggered ability triggers (CR 603.12): `count` is the number its "that many" counts, if it has one.
    fn reflexive(&mut self, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target, count: Option<u16>) {
        // A reflexive triggered ability is an ordinary one (CR 603.12): its intervening "if" must hold for it to trigger at all
        // (CR 603.4), not only as it resolves.
        // Asked as the ability it is, acquired as the resolving one that makes it trigger was (CR 607.5).
        let by = layers::given_link(self.g.cards, &self.g.state, source, ability, self.resolving_acquired(source));
        if let AbilityDef::Triggered(TriggeredDef { condition: Some(condition), .. }) = self.g.cards.ability(ability)
            && !condition(&*self, &Asker { source, controller, by, spell: false })
        {
            return;
        }
        self.g.state.trigger_push(cantrip_state::PendingTrigger {
            source,
            ability,
            controller,
            subject,
            data: [0; 2],
            members: cantrip_state::Span::default(),
            count,
            since: None,
            acquired: self.resolving_acquired(source),
        });
    }

    /// The game state changed with no event to ask the state triggers after (an effect began, an object became a copy): they
    /// are asked now (CR 603.8), or, inside an instruction the card performs as several calls (`Game::one_instruction`), once
    /// it is done - an exchange of control is not judged with one permanent given and the other not yet taken.
    pub(crate) fn state_changed(&mut self) {
        // An attacking or blocking creature that is no longer a creature, and a planeswalker or battle being attacked that is
        // no longer one, leave combat as they stop being one (CR 506.4), not when an event next settles the game; inside an
        // instruction, once it is done (`ops::as_one_instruction`).
        if self.s().settle_held.get() == 0 {
            ops::settle_combat(self);
        }
        let check = &mut self.g.check;
        if check.depth == 0 {
            settle::state_triggers_now(self);
            return;
        }
        check.due = true;
        // A change by an instruction after another's that is not judged yet: the card has not said whether they are one.
        let unsaid = check.done;
        if check.resolving.is_some() && check.depth == 1 {
            check.done = true;
        } else {
            check.open = true;
        }
        if unsaid {
            self.unsaid();
        }
    }

    /// A continuous effect begins (`Game::add_effect`). Its controller is the controller of what made it (CR 611.2a, 113.8,
    /// 603.3a): the replacement effect or mana ability being carried out (`Cx::doer`), else the spell or ability resolving -
    /// whoever controls its source by then (CR 603.7d) - and only for an effect no instruction makes, whoever controls the
    /// source now.
    fn begin_effect(&mut self, source: ObjectId, affected: ObjectId, kind: EffectKind, duration: Duration) {
        // An object that has changed zones is a new object the effect knows nothing of (CR 400.7, 611.2c), and "for as long
        // as [the source] remains on the battlefield" is over before it begins once the source has left (CR 611.2b).
        if !(live(&self.g.state, affected) || kind.outlives_affected()) {
            return;
        }
        // A phased-out permanent is in no set of objects an effect affects, even one naming it (CR 702.26e).
        if self.g.state.obj(affected).zone.is(ZoneKind::PhasedOut) && !kind.outlives_affected() {
            return;
        }
        // "During its controller's next turn" counts from the turn it begins in, which the core reads, never the card.
        let duration = match duration {
            Duration::DuringControllersNextTurn(_) => {
                debug_assert!(
                    matches!(kind, EffectKind::MustAttackAt(_) | EffectKind::CantAttack),
                    "only the declaration of attackers reads whether it is the controller's next turn"
                );
                Duration::DuringControllersNextTurn(self.g.state.turn.number)
            }
            other => other,
        };
        let controller = self.doer.or_else(|| self.effect_controller()).unwrap_or(self.g.state.obj(source).controller);
        let timestamp = self.g.state.timestamp();
        let effect = ContinuousEffect { source, controller, timestamp, affected, kind, duration };
        // A duration already over as the effect would begin never starts, and the effect does nothing (CR 611.2b).
        if !ops::duration_begins(self, &effect) {
            return;
        }
        self.g.state.effect_push(effect);
        // A static ability given to a permanent that affects other objects makes it a source of static abilities.
        if let EffectKind::AddAbility(ability) = kind
            && matches!(self.cards().ability(ability), AbilityDef::Static(s) if !s.own)
        {
            self.g.state.see_statics();
        }
        if matches!(kind, EffectKind::Control(_)) {
            ops::settle_control(self);
        }
        // What it changes may end a "for as long as" (Old Man of the Sea's "that creature's power remains less than or
        // equal to"), and with it the control it gave, at once (CR 611.2b).
        ops::settle_at_once(self);
        self.state_changed();
    }

    /// An action the rules make a point of the game is done (a draw, CR 121.2): what it changed with no event is judged now.
    pub(crate) fn judge_if_due(&mut self) {
        if self.g.check.due {
            settle::state_triggers_now(self);
        }
    }

    /// The state triggers have judged the game as it is now (`StateCheck`): every change so far is seen.
    pub(crate) fn state_judged(&mut self) {
        let StateCheck { depth, resolving, .. } = self.g.check;
        self.g.check = StateCheck { depth, resolving, ..StateCheck::NONE };
    }

    /// Carries out `f` as one instruction (`StateCheck`): what it changes with no event is judged once it is done, and, within
    /// a resolution, is one change however many it makes.
    pub(crate) fn instruction<T>(&mut self, f: impl FnOnce(&mut Self) -> T, done: impl FnOnce(&T) -> bool) -> T {
        self.g.check.depth = self.g.check.depth.saturating_add(1);
        let result = f(self);
        let check = &mut self.g.check;
        check.depth = check.depth.saturating_sub(1);
        if check.resolving.is_some() && check.depth == 1 && std::mem::take(&mut check.open) {
            check.done = true;
        } else if check.depth == 0 && check.due && done(&result) {
            // A run that suspended is redone with its answer, and judged then.
            settle::state_triggers_now(self);
        }
        result
    }

    /// A resolution changed the game with no event twice, and its card said neither that the two are one instruction nor
    /// that an instruction ended between them (`StateCheck`). Under `CANTRIP_VERIFY` (or a game's `verify`) that is refused, naming the card; a game
    /// goes on judging them together.
    #[cold]
    fn unsaid(&self) {
        if self.g.verify || layers::verifying() {
            let name = self.g.check.resolving.map_or("?", |id| self.cards().get(self.g.state.obj(id).card).name);
            panic!(
                "{name}'s resolution changed the game twice with no event between and said neither that the changes are one \
                 instruction (`Game::one_instruction`) nor that an instruction ended between them (`Game::end_sentence`): \
                 the state triggers can't tell when to judge it (CR 603.8, 608.2c)"
            );
        }
    }

    /// An event happened: it is recorded, then the triggers it causes are found - the continuous effects that read the
    /// history seeing it (CR 603.10), each trigger's own question about the past seeing the history as it was just before it
    /// (`Game::history_end`, `cantrip_api::history`).
    pub fn emit(&mut self, event: Event) {
        ops::settle_at_once(self);
        let start = self.g.log.len();
        self.record(event);
        settle::collect_triggers(self, &event, None, start);
    }

    /// Events that happened together (CR 603.2c): triggers are collected over the whole batch, so a "one or more"
    /// ability triggers once. `before` is the battlefield before them and the journal's mark then, for look-back
    /// abilities (CR 603.10a).
    pub fn emit_batch(&mut self, events: &[Event], before: Option<settle::Before>) {
        ops::settle_at_once(self);
        let start = self.g.log.len();
        for &e in events {
            self.record(e);
        }
        settle::collect_batch(self, events, before, start);
    }

    /// Records an event in the history without looking for triggers: the records no ability triggers on.
    pub fn record(&mut self, event: Event) {
        let turn = self.g.state.turn;
        let cause = self.cause();
        let serial = self.g.log.len() as u32;
        self.g.log.push(Record { serial, turn: turn.number, step: turn.step(), active: turn.active, cause, event });
    }

    /// The controller of the spell or ability whose effect is running (CR 101.2): none for a player's own action, the
    /// turn's, or the rules'.
    pub fn effect_controller(&self) -> Option<PlayerId> {
        match self.cause() {
            Cause::Spell(_) | Cause::Ability { .. } => self.resolving().map(|i| i.controller),
            _ => None,
        }
    }

    /// The player doing what card code is doing now (CR 608.2, 609.1): the controller of the replacement effect or mana
    /// ability whose effect is being carried out (`Cx::doer`), else the player whose action the frame is, else the
    /// controller of the spell or ability resolving. What the rules themselves do names its player itself (the turn's lore
    /// counters, CR 714.3c; damage's counters, CR 120.3b-d), so no frame of the rules' own runs card code that puts counters
    /// without one of these; the active player stands in should one ever do, and a debug build stops there.
    pub fn doer(&self) -> PlayerId {
        if let Some(p) = self.doer {
            return p;
        }
        match self.g.state.agenda.get(self.frame).map(|f| f.kind) {
            Some(FrameKind::Act { player }) => player,
            Some(FrameKind::Resolve { .. }) if let Some(item) = self.resolving() => item.controller,
            _ => {
                debug_assert!(false, "card code acted with no player doing it");
                self.g.state.turn.active
            }
        }
    }

    /// Runs `f` as `player`'s doing (`doer`): the effect of a replacement effect or a mana ability they control.
    pub(crate) fn acting_as<T>(&mut self, player: PlayerId, f: impl FnOnce(&mut Self) -> T) -> T {
        let outer = self.doer.replace(player);
        let done = f(self);
        self.doer = outer;
        done
    }

    /// What the frame being run is doing, as an event's cause.
    pub(crate) fn cause(&self) -> Cause {
        let Some(frame) = self.g.state.agenda.get(self.frame) else { return Cause::Rules };
        match frame.kind {
            FrameKind::Act { player } => Cause::Player(player),
            FrameKind::BeginStep | FrameKind::Opening => Cause::Turn,
            FrameKind::Resolve { .. } => match self.resolving() {
                Some(item) if item.kind == StackKind::Spell => Cause::Spell(item.object),
                Some(item) => Cause::Ability { source: item.object, ability: item.ability },
                None => Cause::Rules,
            },
            _ => Cause::Rules,
        }
    }

    /// Holds mana added to a pool as part of the production of a permanent tapped for mana (`Cx::producing`), whoever's pool
    /// it goes to: whether it was, rather than added now.
    fn hold(&mut self, produced: Produced) -> bool {
        match &mut self.producing {
            Some(held) if produced.amount > 0 => {
                held.push(produced);
                true
            }
            _ => false,
        }
    }

    /// The source of mana added now (CR 106.3): the mana ability's source while one runs, else the spell resolving or the
    /// source of the ability resolving; none for a player's own mana ability (CR 605.1a) or anything else.
    pub(crate) fn mana_source(&self) -> ObjectId {
        self.mana_source.or_else(|| self.resolving().map(|i| i.object)).unwrap_or(ObjectId::NONE)
    }

    /// The spell or ability resolving, when the frame being run is its resolution: the item the frame was made for, found by
    /// its serial (`FrameKind::Resolve`) - not the top of the stack, where a spell it casts or a copy it makes goes (CR
    /// 608.2g, 707.10). One that has left the stack as it resolves (Time Spiral exiling itself) is as it last was there.
    pub(crate) fn resolving(&self) -> Option<&StackItem> {
        let FrameKind::Resolve { serial } = self.g.state.agenda.get(self.frame)?.kind else { return None };
        let s = &self.g.state;
        s.stack.iter().rev().find(|i| i.serial == serial).or_else(|| s.left_stack.iter().rev().find(|i| i.serial == serial))
    }

}

impl Query for Game {
    fn state(&self) -> &GameState {
        &self.state
    }

    fn linked_card(&self, card: CardId) -> CardId {
        self.cards.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.cards.linked_to(&self.state, source)
    }

    fn events(&self) -> &[Record] {
        &self.log
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.cards.id_of(def)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.cards.get(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        layers::source_chars(self.cards, &self.state, &self.log, id)
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        layers::token_chars(self.cards, card, copy)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        layers::lost_all_abilities(self.cards, &self.state, &self.log, id)
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        layers::can_target(self.cards, &self.state, &self.log, asker, target)
    }
}

impl Query for Cx<'_> {
    fn state(&self) -> &GameState {
        &self.g.state
    }

    fn linked_card(&self, card: CardId) -> CardId {
        self.g.cards.linked_card(card)
    }

    fn text_linked(&self, source: ObjectId) -> Linked {
        self.g.cards.linked_to(&self.g.state, source)
    }

    fn events(&self) -> &[Record] {
        &self.g.log[..self.g.history_end.unwrap_or(self.g.log.len()).min(self.g.log.len())]
    }

    fn card_id(&self, def: &'static CardDef) -> CardId {
        self.g.cards.id_of(def)
    }

    fn card(&self, id: CardId) -> &'static CardDef {
        self.g.cards.get(id)
    }

    fn chars(&self, id: ObjectId) -> Chars {
        match self.entering {
            Some(e) if e.object == id => e.chars,
            _ => layers::source_chars(self.g.cards, &self.g.state, &self.g.log, id),
        }
    }

    fn token_chars(&self, card: CardId, copy: Option<Copiable>) -> Chars {
        layers::token_chars(self.g.cards, card, copy)
    }

    fn lost_all_abilities(&self, id: ObjectId) -> bool {
        match self.entering {
            Some(e) if e.object == id => e.lost,
            _ => layers::lost_all_abilities(self.g.cards, &self.g.state, &self.g.log, id),
        }
    }

    fn can_target(&self, asker: &Asker, target: Target) -> bool {
        layers::can_target(self.g.cards, &self.g.state, &self.g.log, asker, target)
    }
}

impl Cx<'_> {
    /// The ability in the `nth` place of `source`'s abilities (`Chars::abilities()`), as linked abilities know it where an
    /// effect gave it (`layers::given_link`): what an `Asker` for it asks as (`Asker::by`). None, worked out at no cost, where nothing could
    /// have given anything an ability.
    pub(crate) fn link_nth(&self, source: ObjectId, chars: &Chars, nth: usize) -> Option<Linked> {
        if !layers::acquisition_possible(self.cards(), self.s()) {
            return None;
        }
        let slot = chars.abilities.iter().enumerate().filter(|(_, a)| a.is_some()).nth(nth).map(|(slot, _)| slot)?;
        let ability = chars.abilities[slot]?;
        let acquired = layers::acquired(self.cards(), self.s(), &self.g.log, source, chars, slot);
        layers::given_link(self.cards(), self.s(), source, ability, acquired)
    }

    /// `link_nth` of `source` as it is now, its characteristics worked out only where an effect could have given it an
    /// ability.
    pub(crate) fn link_of_nth(&self, source: ObjectId, nth: usize) -> Option<Linked> {
        if !layers::acquisition_possible(self.cards(), self.s()) {
            return None;
        }
        self.link_nth(source, &self.chars(source), nth)
    }

    /// A spell or ability on the stack, as linked abilities know it (`Query::linked`), where that differs from an ability of
    /// its source's text: what an `Asker` for it asks as (`Asker::by`).
    pub(crate) fn link_of(&self, item: &StackItem) -> Option<Linked> {
        let by = self.linked(item);
        (by != self.g.cards.linked_to(&self.g.state, item.object)).then_some(by)
    }
    /// A question card code asks, as the rules let it be asked (CR 101.2): a discard the resolving spell or ability can't
    /// cause `player` to make is answered with nothing, unasked; and a choice of what to sacrifice takes only the permanents
    /// `player` can sacrifice now (`ops::sacrificeable`: not one another player controls, CR 701.21a, nor one that can't be
    /// sacrificed, nor any when `player` can't be made to sacrifice) - the rulings' "you can't try to sacrifice [it], fail,
    /// and therefore ignore the effect" - answered with
    /// nothing, unasked, when the rule then lets nothing be chosen. A choice of exactly N with fewer that can be then takes
    /// as many as it can (CR 701.21a).
    fn as_allowed(&self, player: PlayerId, mut kind: RequestKind) -> Result<RequestKind, Answer> {
        match &mut kind {
            RequestKind::ChooseCards { purpose: Purpose::Discard, .. }
                if self.effect_controller().is_some_and(|causer| !cantrip_api::may_cause(self, causer, player, CauseKind::Discard)) =>
            {
                Err(Answer::Cards(Vec::new()))
            }
            RequestKind::ChooseCards { purpose: Purpose::Sacrifice, source, cards, rule } => {
                if cards.iter().any(|&c| rule.listed_card(c) && !ops::sacrificeable(self, player, c)) {
                    let among: Vec<ObjectId> = cards.iter().copied().filter(|&c| rule.listed_card(c) && ops::sacrificeable(self, player, c)).collect();
                    if let Count::Exactly(n) = rule.count
                        && among.len() < n as usize
                    {
                        rule.count = Count::AsManyAsCan(n);
                    }
                    rule.among = Some(among);
                }
                // The player's own permanents are nothing to be shown: with none the rule lets be chosen, nothing is asked.
                let none = cantrip_api::select::Choice { chooser: player, source: *source, candidates: cards, selected: &[] };
                if !cantrip_api::select::judge(self, rule, &none).addable.any() {
                    return Err(Answer::Cards(Vec::new()));
                }
                Ok(kind)
            }
            _ => Ok(kind),
        }
    }
}

impl cantrip_api::Game for Cx<'_> {
    fn ask(&mut self, player: PlayerId, kind: RequestKind) -> Result<Answer, Suspend> {
        match self.as_allowed(player, kind) {
            Ok(kind) => self.ask_core(player, kind, false).map_err(|_| Suspend),
            Err(answer) => Ok(answer),
        }
    }

    fn can_sacrifice(&self, player: PlayerId, permanent: ObjectId) -> bool {
        ops::sacrificeable(self, player, permanent)
    }

    fn stage(&self) -> u16 {
        self.g.state.agenda[self.frame].stage
    }

    fn staged_answer(&self) -> Option<Answer> {
        let frame = &self.g.state.agenda[self.frame];
        if frame.staged { frame.answers.first().map(|(_, answer)| answer.clone()) } else { None }
    }

    fn scratch(&self) -> [u32; 4] {
        self.g.state.agenda[self.frame].scratch
    }

    fn set_scratch(&mut self, scratch: [u32; 4]) {
        let at = self.frame;
        self.g.state.update_frame(at, |f| f.scratch = scratch);
    }

    fn still_in_game(&mut self, player: PlayerId) -> bool {
        self.concessions_here();
        !self.g.state.player(player).lost
    }

    fn ask_then(&mut self, next: u16, player: PlayerId, kind: RequestKind) -> Suspend {
        // A question the rules let be asked only in part, or not at all (`as_allowed`): the run goes on at the next stage with
        // the answer they give.
        let kind = match self.as_allowed(player, kind) {
            Ok(kind) => kind,
            Err(answer) => {
                let at = self.frame;
                self.g.state.update_frame(at, |f| {
                    f.staged = true;
                    f.pending = None;
                    f.answers = vec![(None, answer)];
                    f.stage = next;
                });
                return Suspend;
            }
        };
        // A question asked of a player who has left the game (CR 800.4f-h), as `ask_core` treats it: a cost is not paid - the
        // run goes on at the next stage with nothing given - and anything else is asked of the player who makes it instead,
        // once the object's controller has chosen them where they choose.
        let mut player = player;
        if self.g.state.player(player).lost {
            match cantrip_state::stand_in(&self.g.state, &kind, player) {
                cantrip_state::StandIn::Unpaid | cantrip_state::StandIn::NotAsked => {
                    let at = self.frame;
                    self.g.state.update_frame(at, |f| {
                        f.staged = true;
                        f.pending = None;
                        f.answers = vec![(None, Answer::Pass)];
                        f.stage = next;
                    });
                    return Suspend;
                }
                cantrip_state::StandIn::By(other) => player = other,
                cantrip_state::StandIn::Chosen { controller, source, candidates } => {
                    let id = self.g.state.request_id();
                    let at = self.frame;
                    let ask = RequestKind::ChoosePlayer { source, candidates, verb: "makes the choice", amount: 0, subject: None };
                    loops::asked(&mut self.g.state, &ask);
                    self.g.state.update_frame(at, |f| {
                        f.staged = true;
                        f.stand_in = Some(Box::new((next, kind)));
                        f.pending = Some(Request { id, player: controller, kind: ask, may_cancel: false });
                    });
                    return Suspend;
                }
            }
        }
        settle::verify_control(self);
        loops::asked(&mut self.g.state, &kind);
        let id = self.g.state.request_id();
        let at = self.frame;
        self.g.state.update_frame(at, |f| {
            f.staged = true;
            f.next_stage = next;
            f.pending = Some(Request { id, player, kind, may_cancel: false });
        });
        Suspend
    }

    fn draw(&mut self, player: PlayerId, count: u32) {
        ops::draw_cards(self, player, count);
    }

    fn discard(&mut self, card: ObjectId) {
        ops::discard(self, card);
    }

    fn discard_together(&mut self, cards: &[ObjectId]) -> Vec<ObjectId> {
        ops::discard_together(self, cards)
    }

    fn discard_paying(&mut self, cards: &[ObjectId], paying: &[ObjectId]) -> Vec<ObjectId> {
        ops::discard_paying(self, cards, paying)
    }

    fn discard_revealed(&mut self, cards: &[ObjectId]) -> Vec<ObjectId> {
        ops::discard_revealed(self, cards)
    }

    fn create_tokens_together(&mut self, tokens: &[(CardId, PlayerId)]) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_tokens_batch(self, tokens, false, None, &[]).map_err(|_| Suspend)
    }

    fn mill(&mut self, player: PlayerId, count: u32) -> Vec<ObjectId> {
        ops::mill(self, player, count)
    }

    fn mill_together(&mut self, mills: &[(PlayerId, u32)]) -> Vec<ObjectId> {
        ops::mill_together(self, mills)
    }

    fn damage(&mut self, source: ObjectId, target: Target, amount: u16) -> u16 {
        ops::damage(self, source, target, amount, false)
    }

    fn damage_result(&mut self, source: ObjectId, target: Target, amount: u16) -> Option<DamageResult> {
        ops::damage_result(self, source, target, amount, false)
    }

    fn damage_together(&mut self, damage: &[(ObjectId, Target, u16)]) -> Vec<u16> {
        ops::damage_together(self, damage)
    }

    fn each_opponent_loses_life(&mut self, player: PlayerId, amount: i32) -> i32 {
        ops::each_opponent_loses_life(self, player, amount)
    }

    fn destroy_all(&mut self, permanents: &[ObjectId]) -> Vec<ObjectId> {
        ops::destroy_together(self, permanents, ZoneReason::Destroy)
    }

    fn destroy_all_no_regen(&mut self, permanents: &[ObjectId]) -> Vec<ObjectId> {
        ops::destroy_together(self, permanents, ZoneReason::DestroyNoRegen)
    }

    fn sacrifice(&mut self, player: PlayerId, permanents: &[ObjectId]) -> Vec<ObjectId> {
        ops::sacrifice(self, player, permanents)
    }

    fn sacrifice_each(&mut self, sacrifices: &[(PlayerId, ObjectId)]) -> Vec<ObjectId> {
        ops::sacrifice_each(self, sacrifices)
    }

    fn return_to_hand(&mut self, objects: &[ObjectId]) -> Vec<ObjectId> {
        ops::return_to_hand(self, objects)
    }

    fn return_to_hand_revealed(&mut self, cards: &[ObjectId]) -> Vec<ObjectId> {
        ops::return_to_hand_as(self, cards, true)
    }

    fn bring_into_hand(&mut self, cards: &[ObjectId]) -> Vec<ObjectId> {
        ops::bring_into_hand(self, cards)
    }

    fn bring_into_hand_revealed(&mut self, cards: &[ObjectId]) -> Vec<ObjectId> {
        ops::bring_into_hand_as(self, cards, true)
    }

    fn put_into_library(&mut self, objects: &[ObjectId], place: LibraryPlace) -> Vec<ObjectId> {
        ops::put_into_library(self, objects, place)
    }

    fn exile_all(&mut self, objects: &[ObjectId]) -> Vec<ObjectId> {
        ops::exile_all(self, objects)
    }

    fn put_into_graveyard(&mut self, objects: &[ObjectId]) -> Vec<ObjectId> {
        ops::put_into_graveyard(self, objects)
    }

    fn move_to(&mut self, object: ObjectId, to: Zone) -> ObjectId {
        // A phased-out permanent can't be affected by anything (CR 702.26b).
        if self.g.state.obj(object).zone.is(ZoneKind::PhasedOut) {
            return object;
        }
        ops::move_to(self, object, to)
    }

    fn exile_with(&mut self, cards: &[ObjectId], by: Linked, until_leaves: bool) -> Vec<ObjectId> {
        ops::exile_with(self, cards, by, until_leaves)
    }

    fn link_exiled(&mut self, cards: &[ObjectId], by: Linked) {
        ops::link_exiled(self, cards, by);
    }

    fn exile_face_down(&mut self, cards: &[ObjectId], by: Option<Linked>) -> Vec<ObjectId> {
        ops::exile_face_down(self, cards, by)
    }

    fn encode(&mut self, card: ObjectId, creature: ObjectId) -> Option<ObjectId> {
        ops::encode(self, card, creature)
    }

    fn look_at(&mut self, player: PlayerId, cards: &[ObjectId]) {
        ops::look_at(self, player, cards);
    }

    fn put_onto_battlefield_face_down(&mut self, object: ObjectId, controller: PlayerId, how: FaceDown, tapped: bool) -> Result<Option<ObjectId>, Suspend> {
        ops::put_onto_battlefield_face_down(self, object, controller, how, tapped).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield_face_down(&mut self, objects: &[ObjectId], controller: PlayerId, how: FaceDown, tapped: bool) -> Result<Vec<Option<ObjectId>>, Suspend> {
        ops::put_all_onto_battlefield_face_down(self, objects, controller, how, tapped).map_err(|_| Suspend)
    }

    fn turn_face_up(&mut self, permanent: ObjectId, player: PlayerId) {
        ops::turn_face_up(self, permanent, player);
    }

    fn turn_face_up_as(&mut self, permanent: ObjectId, player: PlayerId, megamorph: bool) {
        ops::turn_face_up_as(self, permanent, player, megamorph);
    }

    fn phase_out(&mut self, permanents: &[ObjectId]) {
        self.instruction(|cx| phasing::phase_out(cx, permanents, None, false), |_| true);
    }

    fn phase_out_until_leaves(&mut self, permanent: ObjectId, source: ObjectId, tapped: bool) {
        self.instruction(|cx| phasing::phase_out(cx, &[permanent], Some(source), tapped), |_| true);
    }

    fn unlock_door(&mut self, permanent: ObjectId, door: u8, player: PlayerId) {
        ops::unlock_door(self, permanent, door, player);
    }

    fn remember_chosen_name(&mut self, by: Linked, name: CardId) {
        ops::remember_chosen_name(self, by, name);
    }

    fn move_together(&mut self, moves: &[(ObjectId, Zone)], reason: MoveReason) -> Vec<ObjectId> {
        let moves: Vec<ops::Moving> = moves.iter().map(|&(id, to)| ops::Moving::to(id, to)).collect();
        ops::move_together(self, &moves, reason.into())
    }

    fn move_together_revealed(&mut self, moves: &[(ObjectId, Zone)], reason: MoveReason) -> Vec<ObjectId> {
        let moves: Vec<ops::Moving> = moves.iter().map(|&(id, to)| ops::Moving { revealed: true, ..ops::Moving::to(id, to) }).collect();
        ops::move_together(self, &moves, reason.into())
    }

    fn put_onto_battlefield(&mut self, object: ObjectId, controller: PlayerId, tapped: bool) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield(self, &[(object, controller, tapped)], ZoneReason::Other)
            .map(|v| v.first().copied().unwrap_or(object))
            .map_err(|_| Suspend)
    }

    fn put_onto_battlefield_attacking(&mut self, object: ObjectId, controller: PlayerId, defender: Target, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_attacking(self, object, controller, defender, counters).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_with_aura(&mut self, object: ObjectId, controller: PlayerId, aura: ObjectId) -> Result<ObjectId, Suspend> {
        let new = self.put_onto_battlefield(object, controller, false)?;
        if new != object && live(&self.g.state, aura) && self.g.state.obj(aura).zone.is(ZoneKind::Battlefield) {
            self.g.state.update_obj(new, |o| o.entered_with = Some(aura));
            ops::attach(self, aura, new);
        }
        Ok(new)
    }

    fn put_onto_battlefield_as(&mut self, object: ObjectId, controller: PlayerId, item: &StackItem, what: &'static EntersAs) -> Result<Option<ObjectId>, Suspend> {
        let becomes = Becomes { what, card: item.ability.card };
        ops::put_onto_battlefield_becoming(self, object, controller, becomes).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield_becoming(&mut self, objects: &[(ObjectId, PlayerId)], item: &StackItem, what: &'static EntersAs) -> Result<Vec<ObjectId>, Suspend> {
        let becomes = Becomes { what, card: item.ability.card };
        let all: Vec<(ObjectId, PlayerId, bool)> = objects.iter().map(|&(id, p)| (id, p, false)).collect();
        ops::put_onto_battlefield_becoming_all(self, &all, becomes).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_attached(&mut self, object: ObjectId, controller: PlayerId, host: ObjectId) -> Result<Option<ObjectId>, Suspend> {
        ops::put_onto_battlefield_attached(self, object, controller, host).map_err(|_| Suspend)
    }

    fn sacrifice_and_return(&mut self, player: PlayerId, sacrificed: &[ObjectId], returned: &[(ObjectId, PlayerId)]) -> Result<(), Suspend> {
        ops::sacrifice_and_return(self, player, sacrificed, returned).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield_as(&mut self, objects: &[(ObjectId, PlayerId, bool)]) -> Result<Vec<ObjectId>, Suspend> {
        ops::put_onto_battlefield(self, objects, ZoneReason::Other).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield_with_counters(&mut self, objects: &[(ObjectId, PlayerId, bool)], counters: &[(CounterKind, i16)]) -> Result<Vec<ObjectId>, Suspend> {
        ops::put_onto_battlefield_with(self, objects, ZoneReason::Other, counters).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield(&mut self, objects: &[(ObjectId, PlayerId)]) -> Result<Vec<ObjectId>, Suspend> {
        let all: Vec<(ObjectId, PlayerId, bool)> = objects.iter().map(|&(id, p)| (id, p, false)).collect();
        ops::put_onto_battlefield(self, &all, ZoneReason::Other).map_err(|_| Suspend)
    }

    fn counters_banned(&self, object: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
        static_rules::counters_banned(self, self.cards(), object, controller, kind)
    }

    fn put_onto_battlefield_with_counters(&mut self, object: ObjectId, controller: PlayerId, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_with(self, &[(object, controller, tapped)], ZoneReason::Other, counters)
            .map(|v| v.first().copied().unwrap_or(object))
            .map_err(|_| Suspend)
    }

    fn create_tokens_with_counters(&mut self, def: CardId, controller: PlayerId, count: u32, tapped: bool, counters: &[(CounterKind, i16)], words: Words) -> Result<Vec<ObjectId>, Suspend> {
        let tokens: Vec<(CardId, PlayerId)> = (0..count).map(|_| (def, controller)).collect();
        ops::create_tokens_worded(self, &tokens, tapped, None, counters, words, None).map_err(|_| Suspend)
    }

    fn create_token_copies(&mut self, of: ObjectId, controller: PlayerId, count: u32) -> Result<Vec<ObjectId>, Suspend> {
        let values = self.copiable_values(of);
        ops::create_token_copies(self, values, controller, count).map_err(|_| Suspend)
    }

    fn create_token_copies_of(&mut self, values: Copiable, controller: PlayerId, count: u32) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_token_copies(self, values, controller, count).map_err(|_| Suspend)
    }

    fn create_token_copies_together(&mut self, copies: &[(Copiable, PlayerId)]) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_token_copies_together(self, copies).map_err(|_| Suspend)
    }

    fn create_token_copies_tapped(&mut self, values: Copiable, controller: PlayerId, count: u32, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_token_copies_tapped(self, values, controller, count, tapped).map_err(|_| Suspend)
    }

    fn enter_attacking(&mut self, entered: &[ObjectId], defender: Target) {
        ops::attack_with_tokens(self, entered, defender);
    }

    fn put_onto_battlefield_transformed(&mut self, object: ObjectId, controller: PlayerId, tapped: bool) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_as(self, object, controller, tapped, true, ZoneReason::Other, &[]).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_transformed_with_counters(&mut self, object: ObjectId, controller: PlayerId, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_as(self, object, controller, tapped, true, ZoneReason::Other, counters).map_err(|_| Suspend)
    }

    fn transform(&mut self, permanent: ObjectId) -> Result<(), Suspend> {
        ops::transform(self, permanent, None).map_err(|_| Suspend)
    }

    fn transform_by(&mut self, permanent: ObjectId, item: &StackItem) -> Result<(), Suspend> {
        ops::transform(self, permanent, Some(item)).map_err(|_| Suspend)
    }

    fn create_emblem(&mut self, def: CardId, player: PlayerId) -> ObjectId {
        ops::create_emblem(self, def, player)
    }

    fn create_tokens(&mut self, def: CardId, controller: PlayerId, count: u32, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_tokens(self, def, controller, count, tapped).map_err(|_| Suspend)
    }

    fn copy_spell(&mut self, spell: ObjectId, copier: PlayerId, new_targets: bool) -> Result<Option<ObjectId>, Suspend> {
        ops::copy_spell(self, spell, copier, new_targets).map_err(|_| Suspend)
    }

    fn copy_spell_as(&mut self, spell: ObjectId, copier: PlayerId, new_targets: bool, values: Copiable) -> Result<Option<ObjectId>, Suspend> {
        ops::copy_spell_as(self, spell, copier, new_targets, Some(values)).map_err(|_| Suspend)
    }

    fn copy_ability(&mut self, ability: Target, copier: PlayerId, new_targets: bool) -> Result<Option<u32>, Suspend> {
        ops::copy_ability(self, ability, copier, new_targets).map_err(|_| Suspend)
    }

    fn delayed_trigger_about(&mut self, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, objects: &[ObjectId]) {
        let members = self.g.state.members_push(objects);
        let since = self.g.log.len() as u32;
        let acquired = self.resolving_acquired(source);
        self.g.state.delayed_push(cantrip_state::Delayed { when, source, controller, ability, subject: Target::None, members, since, acquired });
    }

    fn delayed_trigger(&mut self, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target) {
        let since = self.g.log.len() as u32;
        let acquired = self.resolving_acquired(source);
        self.g.state.delayed_push(cantrip_state::Delayed { when, source, controller, ability, subject, members: cantrip_state::Span::default(), since, acquired });
    }

    fn reflexive_trigger(&mut self, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target) {
        self.reflexive(source, controller, ability, subject, None);
    }

    fn reflexive_trigger_counted(&mut self, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target, count: u16) {
        self.reflexive(source, controller, ability, subject, Some(count));
    }

    fn exile_on_leave(&mut self, permanent: ObjectId) {
        self.g.state.update_obj(permanent, |o| o.exile_on_leave = true);
    }

    fn shuffle(&mut self, player: PlayerId) {
        turn::shuffle_library(self, player);
    }

    fn roll_die_modified(&mut self, player: PlayerId, sides: u32, modifier: i32) -> Roll {
        let natural = self.g.state.roll_die(sides);
        let result = natural as i32 + modifier;
        self.emit(Event::DieRolled { player, sides: sides as u8, natural: natural as u8, result: result as i16 });
        Roll { natural, result }
    }

    fn choose_at_random(&mut self, count: usize) -> usize {
        if count <= 1 {
            return 0;
        }
        let index = if count <= 20 { self.g.state.roll_die(count as u32) as usize - 1 } else { self.g.state.random(count as u32) as usize };
        self.emit(Event::ChosenAtRandom { count: count_u32(count), index: count_u32(index) });
        index
    }

    fn flip_coin(&mut self, player: PlayerId) -> bool {
        let won = self.g.state.roll_die(2) == 1;
        self.emit(Event::CoinFlipped { player, result: if won { CoinFlip::Won } else { CoinFlip::Lost } });
        won
    }

    fn flip_coin_for_heads(&mut self, player: PlayerId) -> bool {
        let heads = self.g.state.roll_die(2) == 1;
        self.emit(Event::CoinFlipped { player, result: if heads { CoinFlip::Heads } else { CoinFlip::Tails } });
        heads
    }

    fn random(&mut self, bound: u32) -> u32 {
        if bound <= 1 {
            return 0;
        }
        self.g.state.random(bound)
    }

    fn place_in_library(&mut self, card: ObjectId, from_top: usize) {
        let zone = self.g.state.obj(card).zone;
        if !zone.is(ZoneKind::Library) || !self.g.state.zone_remove(zone, card) {
            return;
        }
        let at = from_top.min(self.g.state.zone(zone).len());
        self.g.state.zone_insert(zone, at, card);
    }

    fn become_prepared(&mut self, permanent: ObjectId) {
        ops::become_prepared(self, permanent);
    }

    fn become_unprepared(&mut self, permanent: ObjectId) {
        ops::become_unprepared(self, permanent);
    }

    fn set_class_level(&mut self, permanent: ObjectId, level: u8) {
        if !live(&self.g.state, permanent) || !self.g.state.obj(permanent).zone.is(ZoneKind::Battlefield) {
            return;
        }
        let was = self.g.state.obj(permanent).class_level.max(1);
        self.g.state.update_obj(permanent, |o| o.class_level = level);
        if level != was {
            self.emit(Event::ClassLevel { object: permanent, level });
        }
    }

    fn arrange_library(&mut self, player: PlayerId, top: &[ObjectId], bottom: &[ObjectId]) {
        ops::arrange_library(self, player, top, bottom);
    }

    fn bottom_in_random_order(&mut self, player: PlayerId, cards: &[ObjectId]) {
        ops::bottom_in_random_order(self, player, cards);
    }

    fn add_combat_phase(&mut self) {
        self.g.state.update_turn(|t| t.extra_combats = t.extra_combats.saturating_add(1));
    }

    fn end_the_turn(&mut self) -> Result<(), Suspend> {
        turn::end_the_turn(self).map_err(|_| Suspend)
    }

    fn lose_game(&mut self, player: PlayerId) {
        ops::lose_game(self, player);
    }

    fn give_experience(&mut self, player: PlayerId, amount: u32) {
        // Counters the effect's controller puts on the player (CR 122.6, 608.2), a replacement of counters being put
        // applying (Vorinclex's "on a permanent or player").
        let by = self.doer();
        counters::put_on_player(self, player, counters::PlayerCounts { experience: amount.min(u16::MAX as u32) as u16, ..Default::default() }, by);
    }

    fn give_poison(&mut self, player: PlayerId, amount: u32) {
        let by = self.doer();
        ops::give_poison(self, player, amount, by);
    }

    fn become_copy(&mut self, source: ObjectId, permanent: ObjectId, values: Copiable, duration: Duration) {
        self.instruction(|cx| ops::become_copy(cx, source, permanent, values, duration), |_| true);
    }

    fn win_game(&mut self, player: PlayerId) {
        ops::win_game(self, player);
    }

    fn add_extra_turns(&mut self, player: PlayerId, count: u32) {
        for _ in 0..count {
            self.add_extra_turn(player);
        }
    }

    fn add_extra_turn(&mut self, player: PlayerId) -> Option<u32> {
        if self.g.state.player(player).lost {
            return None;
        }
        // A number no other extra turn has had (the timestamps only count up), never 0, which names an ordinary turn.
        let id = self.g.state.timestamp() + 1;
        let mut turns = self.g.state.extra_turns.clone();
        turns.push(cantrip_state::ExtraTurn { player, id });
        self.g.state.set_extra_turns(turns);
        Some(id)
    }

    fn gave_gift(&mut self, player: PlayerId, to: PlayerId, gift: Gift) {
        self.emit(Event::GaveGift { player, to, gift });
    }

    fn may_search(&self, searcher: PlayerId, library: PlayerId) -> bool {
        // CR 101.2, 701.23: only a spell or ability can be forbidden to cause a search; outside one there is no causer.
        self.effect_controller().is_none_or(|causer| cantrip_api::may_cause(self, causer, searcher, CauseKind::Search { library }))
    }

    fn searched(&mut self, player: PlayerId, library: PlayerId) {
        self.emit(Event::Searched { player, library });
    }

    fn scried(&mut self, player: PlayerId, count: u32, looked: u32) {
        self.emit(Event::Scried { player, count, looked });
    }

    fn surveilled(&mut self, player: PlayerId, count: u32, looked: u32) {
        self.emit(Event::Surveilled { player, count, looked });
    }

    fn attach(&mut self, attachment: ObjectId, host: ObjectId) -> bool {
        ops::attach(self, attachment, host)
    }

    fn unattach(&mut self, attachment: ObjectId) {
        ops::unattach(self, attachment);
    }

    fn tap(&mut self, permanent: ObjectId) {
        ops::tap(self, permanent);
    }

    fn untap(&mut self, permanent: ObjectId) {
        ops::untap(self, permanent);
    }

    fn exert(&mut self, permanent: ObjectId) {
        ops::exert(self, permanent);
    }

    fn monstrosity(&mut self, permanent: ObjectId, counters: i16) {
        let by = self.doer();
        ops::monstrosity(self, permanent, counters, by);
    }

    fn become_saddled(&mut self, permanent: ObjectId) {
        ops::saddle(self, permanent);
    }

    fn solve(&mut self, permanent: ObjectId) {
        ops::solve(self, permanent);
    }

    fn suspect(&mut self, permanent: ObjectId) {
        ops::suspect(self, permanent);
    }

    fn fight(&mut self, first: ObjectId, second: ObjectId) {
        ops::fight(self, first, second);
    }

    fn create_tokens_attacking(&mut self, tokens: &[(CardId, PlayerId)], tapped: bool, defender: Target) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_tokens_attacking(self, tokens, tapped, defender).map_err(|_| Suspend)
    }

    fn create_tokens_worded(&mut self, tokens: &[(CardId, PlayerId)], tapped: bool, attacking: Option<Target>, words: Words) -> Result<Vec<ObjectId>, Suspend> {
        let created = ops::create_tokens_worded(self, tokens, tapped, None, &[], words, None).map_err(|_| Suspend)?;
        if let Some(defender) = attacking {
            ops::attack_with_tokens(self, &created, defender);
        }
        Ok(created)
    }

    fn create_token_attached(&mut self, def: CardId, controller: PlayerId, host: ObjectId) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_token_attached(self, def, controller, host).map_err(|_| Suspend)
    }

    fn gain_life(&mut self, player: PlayerId, amount: i32) {
        ops::change_life(self, player, amount);
    }

    fn can_gain_life(&self, player: PlayerId) -> bool {
        !static_rules::cant_gain_life(self, self.cards(), player) && !ops::life_total_cant_change(self.s(), player)
    }

    fn lose_life(&mut self, player: PlayerId, amount: i32) {
        ops::change_life(self, player, -amount);
    }

    fn set_life_total(&mut self, player: PlayerId, total: i32) {
        let life = self.g.state.player(player).life;
        ops::change_life(self, player, total - life);
    }

    fn add_mana(&mut self, player: PlayerId, mana: Mana, amount: u32) {
        if self.hold(Produced { to: player, mana, amount, restriction: None, lasting: false }) {
            return;
        }
        let source = self.mana_source();
        self.g.state.add_mana(player, mana, amount, source, None, false);
    }

    fn become_day(&mut self) -> Result<(), Suspend> {
        day_night::become_designation(self, DayNight::Day).map_err(|_| Suspend)
    }

    fn become_night(&mut self) -> Result<(), Suspend> {
        day_night::become_designation(self, DayNight::Night).map_err(|_| Suspend)
    }

    fn note_done(&mut self, source: ObjectId, ability: AbilityRef, player: PlayerId) {
        self.record(Event::Done { source, ability, player });
    }

    fn add_mana_until_end_of_combat(&mut self, player: PlayerId, mana: Mana, amount: u32) {
        if self.hold(Produced { to: player, mana, amount, restriction: None, lasting: true }) {
            return;
        }
        let source = self.mana_source();
        self.g.state.add_mana(player, mana, amount, source, None, true);
    }

    fn add_restricted_mana(&mut self, player: PlayerId, mana: Mana, amount: u32, restriction: ManaRestriction) {
        if self.hold(Produced { to: player, mana, amount, restriction: Some(restriction), lasting: false }) {
            return;
        }
        let source = self.mana_source();
        self.g.state.add_mana(player, mana, amount, source, Some(restriction), false);
    }

    fn add_counters(&mut self, object: ObjectId, kind: CounterKind, amount: i16) {
        // Taking counters off is `remove_counters`; a negative amount still removes them in a release build.
        debug_assert!(amount >= 0, "use remove_counters");
        if amount < 0 {
            counters::remove(self, object, kind, -amount);
            return;
        }
        let by = self.doer();
        counters::put(self, object, kind, amount, by, counters::How::Effect);
    }

    fn add_counters_together(&mut self, puts: &[(ObjectId, CounterKind, i16)]) {
        let by = self.doer();
        // What the one instruction puts on an object is one placement on it, every kind together (CR 122.6, 608.2).
        let mut placements: Vec<counters::Placement> = Vec::with_capacity(puts.len());
        for &(object, kind, amount) in puts.iter().filter(|&&(_, _, amount)| amount > 0) {
            match placements.iter_mut().find(|p| p.object == object) {
                Some(p) => p.counts[kind as usize] += amount,
                None => {
                    let mut counts = [0; CounterKind::COUNT];
                    counts[kind as usize] = amount;
                    placements.push(counters::Placement { object, counts, by });
                }
            }
        }
        counters::put_together(self, &placements, counters::How::Effect);
    }

    fn remove_counters(&mut self, object: ObjectId, kind: CounterKind, amount: i16) -> i16 {
        let before = self.g.state.obj(object).counters(kind);
        counters::remove(self, object, kind, amount);
        before - self.g.state.obj(object).counters(kind)
    }

    fn remove_counters_up_to(&mut self, player: PlayerId, source: ObjectId, object: ObjectId, maximum: u8) -> Result<u16, Suspend> {
        counters::remove_up_to(self, player, source, object, maximum).map_err(|_| Suspend)
    }

    fn remove_a_counter_from_among(&mut self, player: PlayerId, source: ObjectId, permanents: &[ObjectId]) -> Result<bool, Suspend> {
        counters::remove_one_from_among(self, player, source, permanents).map_err(|_| Suspend)
    }

    fn add_effect(&mut self, source: ObjectId, affected: ObjectId, kind: EffectKind, duration: Duration) {
        // One change, though control and the "for as long as" durations it ends are settled with it.
        self.instruction(|cx| cx.begin_effect(source, affected, kind, duration), |_| true);
    }

    fn add_player_effect(&mut self, controller: PlayerId, source: ObjectId, kind: EffectKind, duration: Duration) {
        debug_assert!(kind.outlives_affected(), "a player's effect is about the player, not an object");
        let timestamp = self.g.state.timestamp();
        self.g.state.effect_push(ContinuousEffect { source, controller, timestamp, affected: source, kind, duration });
        self.state_changed();
    }

    fn settle_once(&mut self, instruction: &mut dyn FnMut(&mut dyn cantrip_api::Game)) {
        ops::as_one_instruction(self, |cx| instruction(cx));
    }

    fn counter(&mut self, spell: ObjectId) {
        resolve::counter(self, spell);
    }

    fn counter_to(&mut self, spell: ObjectId, to: CounterTo) {
        resolve::counter_to(self, spell, to);
    }

    fn change_controller(&mut self, spell: ObjectId, player: PlayerId) {
        // One change, though the "for as long as" durations it ends are settled with it.
        self.instruction(|cx| ops::change_spell_controller(cx, spell, player), |_| true);
    }

    fn change_target(&mut self, of: Target, chooser: PlayerId) -> Result<(), Suspend> {
        ops::change_target(self, of, chooser).map_err(|_| Suspend)
    }

    fn change_a_target_to(&mut self, of: Target, to: ObjectId, chooser: PlayerId) -> Result<(), Suspend> {
        ops::change_a_target_to(self, of, to, chooser).map_err(|_| Suspend)
    }

    fn choose_new_targets(&mut self, spell: ObjectId, chooser: PlayerId) -> Result<(), Suspend> {
        ops::choose_new_targets(self, spell, chooser).map_err(|_| Suspend)
    }

    fn remove_spell(&mut self, spell: ObjectId, to: CounterTo) {
        resolve::remove_spell(self, spell, to);
    }

    fn counter_item(&mut self, object: ObjectId, hint: usize) {
        resolve::counter_item(self, object, hint);
    }

    fn reveal(&mut self, player: PlayerId, cards: &[ObjectId]) {
        for &card in cards {
            self.emit(Event::Revealed { player, card });
        }
    }

    fn target_legal(&self, item: &StackItem, slot: usize) -> bool {
        resolve::target_legal(self.g.cards, &self.g.state, &self.g.log, item, slot)
    }

    fn may_pay(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: Cost, question: &'static Question) -> Result<bool, Suspend> {
        actions::may_pay(self, payer, source, subject, &cost, question).map_err(|_| Suspend)
    }

    fn may_pay_x(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: Cost, question: &'static Question) -> Result<Option<u16>, Suspend> {
        actions::may_pay_x(self, payer, source, subject, &cost, question).map_err(|_| Suspend)
    }

    fn may_pay_counted(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, cost: Cost, question: &'static Question) -> Result<Option<u16>, Suspend> {
        actions::may_pay_counted(self, payer, source, subject, &cost, question).map_err(|_| Suspend)
    }

    fn may_pay_times(&mut self, payer: PlayerId, source: ObjectId, subject: ObjectId, mana: ManaCost, most: u16, question: &'static Question) -> Result<u16, Suspend> {
        actions::may_pay_times(self, payer, source, subject, mana, most, question).map_err(|_| Suspend)
    }

    fn could_cast_during_resolution(&mut self, card: ObjectId) -> bool {
        let Some(player) = self.resolving().map(|i| i.controller) else { return false };
        actions::could_cast_during_resolution(self, card, player)
    }

    fn could_cast_during_resolution_by(&mut self, card: ObjectId, player: PlayerId) -> bool {
        actions::could_cast_during_resolution(self, card, player)
    }

    fn could_play_land_during_resolution(&self, card: ObjectId) -> bool {
        actions::could_play_land_during_resolution(self, card)
    }

    fn play_land_during_resolution(&mut self, card: ObjectId) -> Result<Option<ObjectId>, Suspend> {
        actions::play_land_during_resolution(self, card).map_err(|_| Suspend)
    }

    fn cast_during_resolution(&mut self, card: ObjectId, instead: Option<ManaCost>) -> Result<bool, Suspend> {
        actions::cast_during_resolution(self, card, CastTerms { instead, ..Default::default() }).map_err(|_| Suspend)
    }

    fn cast_ways_during_resolution(&mut self, card: ObjectId) -> Vec<CastWay> {
        let Some(player) = self.resolving().map(|i| i.controller) else { return Vec::new() };
        actions::cast_ways_during_resolution(self, card, player)
    }

    fn cast_ways_during_resolution_by(&mut self, card: ObjectId, player: PlayerId) -> Vec<CastWay> {
        actions::cast_ways_during_resolution(self, card, player)
    }

    fn cast_during_resolution_as(&mut self, card: ObjectId, terms: CastTerms) -> Result<bool, Suspend> {
        actions::cast_during_resolution(self, card, terms).map_err(|_| Suspend)
    }

    fn create_copy_in_exile(&mut self, of: ObjectId, player: PlayerId) -> ObjectId {
        ops::copy_card_into(self, of, player, Zone::exile(player))
    }

    fn allow_play(&mut self, card: ObjectId, player: PlayerId, grant: PlayGrant) {
        let turn = self.g.state.turn.number;
        // What gave it (CR 601.3): the spell or ability resolving, which a cast or land play by it is recorded as made by.
        let by = match self.cause() {
            Cause::Spell(spell) => Grant { object: spell, ability: AbilityRef::spell(self.g.state.obj(spell).card) },
            Cause::Ability { source, ability } => Grant { object: source, ability },
            _ => Grant { object: card, ability: AbilityRef::spell(self.g.state.obj(card).card) },
        };
        let permission = PlayPermission { card, player, grant, by, turn };
        // A "for as long as you control" duration that is already over as it would begin does nothing (CR 611.2b).
        if self.g.state.permission_lasts(&permission) {
            self.g.state.permission_push(permission);
        }
    }

    fn could_cast_transformed_during_resolution(&mut self, card: ObjectId) -> bool {
        actions::could_cast_transformed_during_resolution(self, card)
    }

    fn cast_transformed_during_resolution(&mut self, card: ObjectId) -> Result<bool, Suspend> {
        actions::cast_transformed_during_resolution(self, card).map_err(|_| Suspend)
    }

    fn creatures_cant_block(&mut self) {
        let number = self.g.state.turn.number;
        self.g.state.update_turn(|t| t.no_blocks_turn = number);
    }

    fn become_monarch(&mut self, player: PlayerId) {
        designations::become_monarch(self, player);
    }

    fn take_initiative(&mut self, player: PlayerId) {
        designations::take_initiative(self, player);
    }

    fn venture(&mut self, player: PlayerId) -> Result<(), Suspend> {
        designations::venture(self, player).map_err(|_| Suspend)
    }

    fn increase_speed(&mut self, player: PlayerId, amount: u8) {
        speed::increase(self, player, amount);
    }

    fn damage_cant_be_prevented(&mut self) {
        let number = self.g.state.turn.number;
        self.g.state.update_turn(|t| t.no_prevention_turn = number);
    }

    fn allow_extra_loyalty_activations(&mut self, permanent: ObjectId, times: u8) {
        let player = self.g.state.obj(permanent).controller;
        let ability = AbilityRef { card: self.g.state.obj(permanent).card, index: actions::EXTRA_LOYALTY_ALLOWED };
        for _ in 0..times {
            self.record(Event::AbilityActivated { source: permanent, ability, player, by: Grant::NONE });
        }
    }

    fn change_energy(&mut self, player: PlayerId, delta: i32) {
        let by = self.doer();
        ops::change_energy(self, player, delta, by);
    }

    fn proliferate(&mut self, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
        ops::proliferate(self, player, source).map_err(|_| Suspend)
    }

    fn set_copy(&mut self, object: ObjectId, copy: Copiable) {
        self.g.state.update_obj(object, |o| o.copy = Some(copy));
        if self.cards().statics[copy.card.0 as usize] {
            self.g.state.see_statics();
        }
        self.state_changed();
    }

    fn end_sentence(&mut self) {
        self.judge_if_due();
    }

    fn one_instruction(&mut self, f: &mut dyn FnMut(&mut dyn cantrip_api::Game) -> Result<(), Suspend>) -> Result<(), Suspend> {
        self.instruction(|cx| f(cx), Result::is_ok)
    }
}

/// The ContinuousEffect type card code builds through `add_effect`.
pub use cantrip_state::ContinuousEffect;

#[allow(dead_code)]
fn _assert_rng_is_state(_: Rng) {}

/// Whether `player` may not see what `id` is: a card in a library, in another player's hand or sideboard, or face down
/// where they have not seen it (CR 401.2, 402.3, 406.3, 708.5).
fn hidden_from(state: &GameState, player: PlayerId, id: ObjectId) -> bool {
    let obj = state.obj(id);
    match obj.zone.kind() {
        ZoneKind::Library => true,
        ZoneKind::Hand | ZoneKind::Sideboard => obj.zone.player != player,
        // A face-down permanent or spell: its controller may look at it (CR 708.5).
        ZoneKind::Battlefield | ZoneKind::PhasedOut | ZoneKind::Stack => obj.face_down && obj.controller != player,
        _ => obj.face_down && obj.seen_by & (1 << player.0) == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zero-cost 1/1, to cast and attack with.
    static BODY: CardDef = CardDef {
        name: "Fork Body",
        types: TypeSet::of(&[CardType::Creature]),
        power: Some(1),
        toughness: Some(1),
        ..CardDef::BLANK
    };

    static OTHER: CardDef = CardDef { name: "Fork Other", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

    /// A real deal (shuffled, mulligans asked) of twenty bodies each, checked as it plays.
    fn deal() -> Game {
        let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&BODY])));
        let deck = vec![cards.id_of(&BODY); 20];
        let mut game = Game::new(cards, &Setup { decks: vec![deck.clone(), deck], seed: 7, shuffle: true, ..Setup::default() });
        game.verify = true;
        game.run();
        game
    }

    /// The same answer to the same question every time: keep, cast what is free, attack with everything, never block.
    fn answer_for(request: &Request) -> Answer {
        match &request.kind {
            RequestKind::Mulligan { .. } => Answer::No,
            RequestKind::ChooseStartingPlayer { candidates } => Answer::Player(candidates[0]),
            RequestKind::Priority { options } => match options.iter().find(|o| o.kind == ActionKind::CastSpell && o.targets_required == 0) {
                Some(cast) => Answer::Act(Action::of(cast)),
                None => Answer::Pass,
            },
            RequestKind::DeclareAttackers { eligible, players, .. } => Answer::Attack(eligible.iter().map(|&a| (a, Target::Player(players[0]))).collect()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand[..*count as usize].to_vec()),
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..sources.len() as u32).collect()),
            other => panic!("no answer for {other:?}"),
        }
    }

    /// Plays up to `n` answers, and gives them back.
    fn play(game: &mut Game, n: usize) -> Vec<(u32, PlayerId, Answer)> {
        let mut given = Vec::new();
        for _ in 0..n {
            let Some(request) = game.pending().cloned() else { break };
            let answer = answer_for(&request);
            game.answer(request.id, request.player, answer.clone()).expect("legal");
            given.push((request.id, request.player, answer));
        }
        given
    }

    fn replay(game: &mut Game, answers: &[(u32, PlayerId, Answer)]) {
        for (request, player, answer) in answers {
            game.answer(*request, *player, answer.clone()).expect("legal");
        }
    }

    /// A fork plays on, goes back to its root and plays the same answers again to the same position, as a second fork and
    /// the real game do; the real game is untouched by any of it, and the fork holds only the journal's tail.
    #[test]
    fn a_game_forked_for_search_plays_on_and_comes_back_as_the_game_would() {
        let mut game = deal();
        while game.state.turn.number < 3 {
            assert!(!play(&mut game, 1).is_empty());
        }
        let (state, log, entries, nodes, at) = (game.state.clone(), game.log.clone(), game.state.journal.entries.len(), game.tree.len(), game.at);

        let mut fork = game.fork_for_search();
        let root = fork.checkpoint();
        assert!(root.0 as usize >= nodes, "the fork's ids begin past the game's");
        assert!(fork.state.journal.entries.len() < entries / 2, "the fork holds the journal's tail, not the game's");
        assert!(fork.state == state && fork.log == log);
        let answers = play(&mut fork, 40);
        assert!(answers.len() == 40 && fork.state.turn.number > state.turn.number);
        let (after, after_log) = (fork.state.clone(), fork.log.clone());

        fork.goto(root);
        assert!(fork.state == state && fork.log == log, "back at the root of the fork");
        // Another branch, then the same answers again: the tree's nodes are found and gone back down.
        play(&mut fork, 3);
        fork.goto(root);
        replay(&mut fork, &answers);
        assert!(fork.state == after && fork.log == after_log);

        let mut again = game.fork_for_search();
        replay(&mut again, &answers);
        assert!(again.state == after && again.log == after_log, "a second fork plays the answers out to the same position");

        assert!(game.state == state && game.log == log && game.tree.len() == nodes && game.at == at, "the real game is untouched");
        assert_eq!(game.state.journal.entries.len(), entries);
        replay(&mut game, &answers);
        assert!(game.state == after && game.log == after_log, "the real game plays the same answers to the same position");
    }

    /// A fork cannot go back into the true past: the game's own nodes are before its root.
    #[test]
    #[should_panic(expected = "before the root of a game forked for search")]
    fn a_fork_cannot_go_to_a_node_before_its_root() {
        let mut game = deal();
        play(&mut game, 10);
        let past = game.checkpoint();
        play(&mut game, 1);
        let mut fork = game.fork_for_search();
        fork.goto(past);
    }

    /// A reseeded fork draws from its own generator, and rolling back past a draw in the journal's tail does not bring the
    /// game's back.
    #[test]
    fn a_reseeded_fork_keeps_its_own_generator_through_its_journal() {
        let mut game = deal();
        play(&mut game, 10);
        let mut fork = game.fork_for_search();
        // A draw from the game's generator in the tail, before the reseed.
        fork.state.random(1000);
        fork.reseed(99);
        let seeded = fork.state.rng;
        assert_ne!(seeded, game.state.rng);
        let floor = Mark(fork.state.journal.base);
        fork.state.random(1000);
        fork.state.rollback(floor);
        assert_eq!(fork.state.rng, seeded, "every generator entry of the fork's journal is its own");
    }

    /// A shuffled library is dealt in the order shuffling the decklist's cards in place would have given, the generator drawing
    /// as it did, and its objects are numbered in that order: an id says where a card was dealt, not where it was listed.
    #[test]
    fn a_shuffled_library_is_dealt_in_shuffled_order_with_ids_in_that_order() {
        let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&BODY, &OTHER])));
        let decks: Vec<Vec<CardId>> = (0..2).map(|seat| (0..30).map(|i| cards.id_of(if (i * 7 + seat) % 3 == 0 { &OTHER } else { &BODY })).collect()).collect();
        let game = Game::new(cards, &Setup { decks: decks.clone(), seed: 7, shuffle: true, ..Setup::default() });
        // As it was dealt before: objects numbered in decklist order, each library shuffled in place, then the chooser drawn.
        let mut before = GameState::new(2, 7);
        for (seat, deck) in decks.iter().enumerate() {
            for &card in deck {
                let id = before.push_obj(Obj::new(card, PlayerId(seat as u8), Zone::library(PlayerId(seat as u8))));
                before.zone_push(Zone::library(PlayerId(seat as u8)), id);
            }
        }
        for p in before.seats() {
            turn::shuffle(&mut before, Zone::library(p));
        }
        let chooser = PlayerId(before.random(2) as u8);
        for p in game.state.seats() {
            let library = game.state.zone(Zone::library(p));
            let order = |s: &GameState, z: &[ObjectId]| z.iter().map(|&id| s.obj(id).card).collect::<Vec<_>>();
            assert_eq!(order(&game.state, library), order(&before, before.zone(Zone::library(p))));
            assert!(library.windows(2).all(|w| w[0] < w[1]), "ids in library order");
        }
        assert_eq!(game.state.turn.starting, chooser);
        assert_eq!(game.state.rng, before.rng);
    }
}
