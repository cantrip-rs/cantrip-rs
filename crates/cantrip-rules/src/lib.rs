//! The core: rules authority and timing (docs/engine-design.md, "Layers: core, shared mechanics, cards").
//!
//! The engine is a step function. [`Game::run`] works the agenda until a frame asks something or the game ends;
//! [`Game::answer`] gives the answer and runs on. Between the two, the game is plain data: copy it with `clone`.

pub mod tree;
mod actions;
mod attach;
mod day_night;
mod designations;
mod layers;
mod loops;
mod ops;
mod phasing;
mod replace;
mod resolve;
mod settle;
mod speed;
mod static_rules;
mod turn;
mod validate;

use cantrip_api::*;
use cantrip_state::{Frame, FrameKind, Mark, Marks, Rng};

pub use actions::{activation_costs, cast_costs, cast_total, describe, exile_cast_texts, granted_cast_cost, life_for_cast, granted_cast_total, special_action_costs, sacrifices_for_reduction};
pub use validate::{check as check_answer, distribution_problem, name_problem};
pub use layers::{chars, live, summoning_sick};

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
    /// Check that every rollback restores the state exactly (slow; for tests).
    pub verify: bool,
}

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
        }
    }
}

/// What dealing a game needs.
#[derive(Clone, Debug, Default)]
pub struct Setup {
    /// Each seat's deck, top of the library first.
    pub decks: Vec<Vec<CardId>>,
    /// Each seat's sideboard: the cards it owns outside the game (CR 100.4, 400.11a, 108.3b), which effects that bring a
    /// card into the game from outside it choose among. A seat with no entry has none.
    pub sideboards: Vec<Vec<CardId>>,
    /// In a Limited game, the basic land cards the event supplies: a Limited sideboard holds any number of each (Magic
    /// Tournament Rules; North Wind Avatar's ruling), so each seat always has one of each outside the game. Empty otherwise.
    pub basic_lands_outside: Vec<CardId>,
    pub seed: u64,
    /// Shuffle the libraries and play the real start of a game (CR 103): who goes first, mulligans. Without it the
    /// libraries stay in the order given, the first seat goes first and both opening hands are dealt and kept, as a
    /// scenario wants.
    pub shuffle: bool,
    /// With `shuffle` off: still ask who goes first and offer mulligans (a scenario's `[Opening]`), this seat choosing.
    pub opening_chooser: Option<PlayerId>,
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
                let id = state.push_obj(Obj {
                    card,
                    owner: p,
                    controller: p,
                    zone,
                    token: false,
                    tapped: false,
                    controlled_since: 0,
                    damage: 0,
                    deathtouched: false,
                    counters: [0; CounterKind::COUNT],
                    timestamp: 0,
                    copy: None,
                    loyalty_turn: 0,
                    became: None,
                    exile_on_leave: false,
                    kicked: 0,
                    paid: 0,
                    gift: None,
                    spent: ManaPool::default(),
                    cast_way: 0,
                    class_level: 0,
                    prepared: false,
                    card_copy: false,
                    prepared_from: None,
                    attached_to: None,
                    attached_player: None,
                    entered_with: None,
                    chosen_color: None,
                    chosen_subtype: None,
                    on_adventure: false,
                    cast_from: None,
                    activated_turn: 0,
                    activated_mask: 0,
                    x: 0,
                    marks: Marks::default(),
                    transformed: false,
                    exiled: None,
                    face_down: false,
                    seen_by: 0,
                });
                state.zone_push(zone, id);
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

        let mut game = Game { state, log: Vec::new(), tree: tree::Tree::new(), at: tree::NodeId::ROOT, cards, verify: false };
        if real {
            game.state.frame_push(Frame::new(FrameKind::Opening));
        } else {
            // A scenario's deal: seven each, in turn order, kept.
            for p in game.state.apnap().collect::<Vec<_>>() {
                let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None };
                for _ in 0..7 {
                    ops::draw(&mut cx, p);
                }
                game.state.update_player(p, |pl| pl.kept = true);
            }
            game.log.clear();
            // CR 103.6a: a card in a hand that may begin the game on the battlefield is asked before the first turn.
            let begins = game.state.seats().any(|p| {
                game.state.zone(Zone::hand(p)).iter().any(|&id| cards.get(game.state.obj(id).card).opening_hand.is_some())
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

            let mark = self.state.mark();
            let log_mark = self.log.len() as u32;
            {
                let frame = &mut self.state.agenda[at];
                frame.mark = mark.0;
                frame.log_mark = log_mark;
            }
            let before = if self.verify { Some(self.state.clone()) } else { None };

            let kind = self.state.agenda[at].kind;
            let (flow, asked_at) = {
                let mut cx = Cx { g: self, frame: at, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None };
                let flow = match kind {
                    FrameKind::Opening => turn::opening(&mut cx),
                    FrameKind::BeginStep => turn::begin_step(&mut cx),
                    FrameKind::Priority => turn::priority(&mut cx),
                    FrameKind::Settle => settle::settle(&mut cx),
                    FrameKind::Act { player } => actions::act(&mut cx, player),
                    FrameKind::Resolve => resolve::resolve(&mut cx),
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
                other => other,
            };
            match flow {
                Ok(Flow::Done) => {
                    self.state.frame_remove(at);
                }
                Ok(Flow::Again) => {
                    if !self.state.agenda[at].answers.is_empty() {
                        self.state.update_frame(at, |f| f.answers.clear());
                    }
                }
                Err(Stop::Suspend) => {
                    if let Some(before) = before {
                        // Rolling back the run must give back exactly the state it started from.
                        let mut probe = self.state.clone_with_journal();
                        probe.rollback(mark);
                        assert!(probe == before, "a rollback did not restore the state: some write was not journalled");
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
        // A loop being watched keeps the answers given to its questions (`loops`), as they were asked.
        let watched = loops::answering(&self.state, request, &pending, &answer);
        let here = self.marks();
        self.tree.close(self.at, here);
        // A run answered without stages is redone from its start, so the events it had recorded are thrown away and
        // the child's begin where the run did, the child keeping them to give back to its parent; the journal only grows (a
        // revert appends the inverse entries).
        let (child_start, displaced) = if self.state.agenda[at].staged {
            (here, Vec::new())
        } else {
            let log_mark = self.state.agenda[at].log_mark as usize;
            ((here.0, log_mark), self.log[log_mark..].to_vec())
        };
        let child = self.tree.add(self.at, (request, player, answer.clone()), child_start, displaced);

        let frame = &self.state.agenda[at];
        if frame.staged {
            let next = frame.next_stage;
            self.state.update_frame(at, |f| {
                f.pending = None;
                f.answers = vec![answer];
                f.stage = next;
            });
        } else {
            let mark = Mark(frame.mark);
            let log_mark = frame.log_mark as usize;
            let mut answers = frame.answers.clone();
            answers.push(answer);
            self.state.revert(mark);
            self.log.truncate(log_mark);
            self.state.update_frame(at, |f| f.answers = answers);
        }
        if let Some(watched) = watched {
            loops::answered(&mut self.state, player, matches!(pending.kind, RequestKind::Priority { .. }), watched);
        }
        self.run();
        self.at = child;
        if self.verify {
            // Up to the node the answer was given at and back: the working state must come back exactly.
            let (state, log) = (self.state.clone(), self.log.clone());
            let parent = self.tree.parent(child).expect("a parent");
            self.goto(parent);
            assert_eq!(self.log.len(), here.1, "going back up to the node the answer was given at did not restore its history");
            self.goto(child);
            assert!(self.state == state && self.log == log, "moving through the game tree did not restore the position");
        }
        Ok(())
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
        Game { state, log: self.log.clone(), tree, at: root, cards: self.cards, verify: self.verify }
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

    /// Whether `player` can't gain life right now (CR 119.7): an effect or a static ability on the battlefield says so.
    pub fn cant_gain_life(&self, player: PlayerId) -> bool {
        static_rules::cant_gain_life(self, self.cards, player)
    }

    /// Whether a static ability on the battlefield gives `player` the keyword ability `keyword` right now (CR 702.18a: "You
    /// have shroud").
    pub fn player_has(&self, player: PlayerId, keyword: Keyword) -> bool {
        layers::player_has(self.cards, &self.state, &self.log, player, keyword)
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

    pub fn ask_core(&mut self, player: PlayerId, kind: RequestKind, may_cancel: bool) -> R<Answer> {
        // A real choice, asked or answered on a re-run, means no loop through here is a mandatory one (CR 104.4b); and a loop
        // of choices is watched, and its players made to choose otherwise (CR 732.3).
        let replaying = self.cursor < self.g.state.agenda[self.frame].answers.len();
        let (kind, key) = match loops::asked(self, player, kind, true, replaying) {
            loops::Asked::Ask { kind, key } => (kind, key),
            loops::Asked::Answered(answer) => return Ok(answer),
        };
        let frame = &self.g.state.agenda[self.frame];
        if self.cursor < frame.answers.len() {
            let answer = frame.answers[self.cursor].clone();
            self.cursor += 1;
            self.shown(player, &kind);
            return Ok(answer);
        }
        // A question is already waiting: an operation that cannot carry the suspension out (a zone change asking
        // which replacement applies first) went on with the first, and the run is abandoned and replayed once the
        // answer is in (see `Game::run`). No second question replaces it.
        if frame.pending.is_some() {
            return Err(Stop::Suspend);
        }
        let id = self.g.state.request_id();
        let at = self.frame;
        self.asked_at = Some((self.g.state.mark(), self.g.log.len()));
        self.g.state.update_frame(at, |f| f.pending = Some(Request { id, player, kind, may_cancel }));
        loops::waiting(&mut self.g.state, id, key);
        Err(Stop::Suspend)
    }

    /// The hidden cards a question showed its player, recorded as it is answered (`Event::Shown`): the cards a scry,
    /// surveil or look at the top looked at, a library searched, a hand chosen from. Only the questions that show cards;
    /// priority and combat name nothing hidden.
    fn shown(&mut self, player: PlayerId, kind: &RequestKind) {
        if !matches!(
            kind,
            RequestKind::DistributeCards { .. } | RequestKind::SearchLibrary { .. } | RequestKind::ChooseCards { .. } | RequestKind::ChooseName { .. } | RequestKind::ChooseTarget { .. }
        ) {
            return;
        }
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
        if let AbilityDef::Triggered(TriggeredDef { condition: Some(condition), .. }) = self.g.cards.ability(ability)
            && !condition(&*self, &Asker { source, controller })
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
        });
    }

    /// An event happened: the triggers it causes are found, then it is recorded (so a trigger or replacement
    /// deciding about it sees the history as it was just before it, `cantrip_api::history`).
    pub fn emit(&mut self, event: Event) {
        settle::collect_triggers(self, &event, None);
        self.record(event);
    }

    /// Events that happened together (CR 603.2c): triggers are collected over the whole batch, so a "one or more"
    /// ability triggers once. `before` is the battlefield before them, for look-back abilities.
    pub fn emit_batch(&mut self, events: &[Event], before: Option<&[ObjectId]>) {
        settle::collect_batch(self, events, before);
        for &e in events {
            self.record(e);
        }
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
            Cause::Spell(_) | Cause::Ability { .. } => self.g.state.stack.last().map(|i| i.controller),
            _ => None,
        }
    }

    /// The player doing what the running frame does (CR 122.6a): who a player's action is, or who controls the spell or
    /// ability that is resolving. None for the rules' own doing.
    pub fn actor(&self) -> Option<PlayerId> {
        match self.cause() {
            Cause::Player(p) => Some(p),
            Cause::Spell(_) | Cause::Ability { .. } => self.g.state.stack.last().map(|i| i.controller),
            _ => None,
        }
    }

    /// What the frame being run is doing, as an event's cause.
    pub(crate) fn cause(&self) -> Cause {
        let Some(frame) = self.g.state.agenda.get(self.frame) else { return Cause::Rules };
        match frame.kind {
            FrameKind::Act { player } => Cause::Player(player),
            FrameKind::BeginStep | FrameKind::Opening => Cause::Turn,
            FrameKind::Resolve => match self.resolving() {
                Some(item) if item.kind == StackKind::Spell => Cause::Spell(item.object),
                Some(item) => Cause::Ability { source: item.object, ability: item.ability },
                None => Cause::Rules,
            },
            _ => Cause::Rules,
        }
    }

    /// The source of mana added now (CR 106.3): the mana ability's source while one runs, else the spell resolving or the
    /// source of the ability resolving; none for a player's own mana ability (CR 605.1a) or anything else.
    pub(crate) fn mana_source(&self) -> ObjectId {
        self.mana_source.or_else(|| self.resolving().map(|i| i.object)).unwrap_or(ObjectId::NONE)
    }

    /// The spell or ability resolving, when the frame being run is its resolution: the top of the stack.
    pub(crate) fn resolving(&self) -> Option<&StackItem> {
        let frame = self.g.state.agenda.get(self.frame)?;
        if frame.kind != FrameKind::Resolve {
            return None;
        }
        self.g.state.stack.last()
    }

}

impl Query for Game {
    fn state(&self) -> &GameState {
        &self.state
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
        layers::chars(self.cards, &self.state, &self.log, id)
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

    fn events(&self) -> &[Record] {
        &self.g.log
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
            _ => layers::chars(self.g.cards, &self.g.state, &self.g.log, id),
        }
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

impl cantrip_api::Game for Cx<'_> {
    fn ask(&mut self, player: PlayerId, kind: RequestKind) -> Result<Answer, Suspend> {
        // CR 101.2: a discard or sacrifice an opponent's spell or ability can't cause is not offered as a choice.
        if let RequestKind::ChooseCards { purpose: purpose @ (Purpose::Discard | Purpose::Sacrifice), .. } = &kind
            && let Some(causer) = self.effect_controller()
            && !cantrip_api::may_cause_discard_or_sacrifice(&*self, causer, player, *purpose == Purpose::Sacrifice)
        {
            return Ok(Answer::Cards(Vec::new()));
        }
        self.ask_core(player, kind, false).map_err(|_| Suspend)
    }

    fn stage(&self) -> u16 {
        self.g.state.agenda[self.frame].stage
    }

    fn staged_answer(&self) -> Option<Answer> {
        let frame = &self.g.state.agenda[self.frame];
        if frame.staged { frame.answers.first().cloned() } else { None }
    }

    fn scratch(&self) -> [u32; 4] {
        self.g.state.agenda[self.frame].scratch
    }

    fn set_scratch(&mut self, scratch: [u32; 4]) {
        let at = self.frame;
        self.g.state.update_frame(at, |f| f.scratch = scratch);
    }

    fn ask_then(&mut self, next: u16, player: PlayerId, kind: RequestKind) -> Suspend {
        let (kind, key) = match loops::asked(self, player, kind, false, false) {
            loops::Asked::Ask { kind, key } => (kind, key),
            loops::Asked::Answered(_) => unreachable!("a staged question is always asked"),
        };
        let id = self.g.state.request_id();
        let at = self.frame;
        self.g.state.update_frame(at, |f| {
            f.staged = true;
            f.next_stage = next;
            f.pending = Some(Request { id, player, kind, may_cancel: false });
        });
        loops::waiting(&mut self.g.state, id, key);
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

    fn sacrifice(&mut self, permanents: &[ObjectId]) -> Vec<ObjectId> {
        ops::sacrifice(self, permanents)
    }

    fn return_to_hand(&mut self, objects: &[ObjectId]) -> Vec<ObjectId> {
        ops::return_to_hand(self, objects)
    }

    fn bring_into_hand(&mut self, cards: &[ObjectId]) -> Vec<ObjectId> {
        ops::bring_into_hand(self, cards)
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

    fn exile_with(&mut self, cards: &[ObjectId], by: ObjectId, until_leaves: bool) -> Vec<ObjectId> {
        ops::exile_with(self, cards, by, until_leaves)
    }

    fn link_exiled(&mut self, cards: &[ObjectId], by: ObjectId) {
        ops::link_exiled(self, cards, by);
    }

    fn exile_face_down(&mut self, cards: &[ObjectId], by: Option<ObjectId>) -> Vec<ObjectId> {
        ops::exile_face_down(self, cards, by)
    }

    fn encode(&mut self, card: ObjectId, creature: ObjectId) -> Option<ObjectId> {
        ops::encode(self, card, creature)
    }

    fn look_at(&mut self, player: PlayerId, cards: &[ObjectId]) {
        ops::look_at(self, player, cards);
    }

    fn put_onto_battlefield_face_down(&mut self, object: ObjectId, controller: PlayerId) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_face_down(self, object, controller).map_err(|_| Suspend)
    }

    fn turn_face_up(&mut self, permanent: ObjectId, player: PlayerId) {
        ops::turn_face_up(self, permanent, player);
    }

    fn phase_out(&mut self, permanents: &[ObjectId]) {
        phasing::phase_out(self, permanents, None, false);
    }

    fn phase_out_until_leaves(&mut self, permanent: ObjectId, source: ObjectId, tapped: bool) {
        phasing::phase_out(self, &[permanent], Some(source), tapped);
    }

    fn unlock_door(&mut self, permanent: ObjectId, door: u8, player: PlayerId) {
        ops::unlock_door(self, permanent, door, player);
    }

    fn remember_chosen_name(&mut self, permanent: ObjectId, name: CardId) {
        ops::remember_chosen_name(self, permanent, name);
    }

    fn move_together(&mut self, moves: &[(ObjectId, Zone)], reason: ZoneReason) -> Vec<ObjectId> {
        let moves: Vec<ops::Moving> = moves.iter().map(|&(id, to)| ops::Moving::to(id, to)).collect();
        ops::move_together(self, &moves, reason)
    }

    fn put_onto_battlefield(&mut self, object: ObjectId, controller: PlayerId, tapped: bool) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield(self, &[(object, controller, tapped)], ZoneReason::Other)
            .map(|v| v.first().copied().unwrap_or(object))
            .map_err(|_| Suspend)
    }

    fn put_onto_battlefield_attacking(&mut self, object: ObjectId, controller: PlayerId, defender: Target) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_attacking(self, object, controller, defender).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_with_aura(&mut self, object: ObjectId, controller: PlayerId, aura: ObjectId) -> Result<ObjectId, Suspend> {
        let new = self.put_onto_battlefield(object, controller, false)?;
        if new != object && live(&self.g.state, aura) && self.g.state.obj(aura).zone.is(ZoneKind::Battlefield) {
            self.g.state.update_obj(new, |o| o.entered_with = Some(aura));
            ops::attach(self, aura, new);
        }
        Ok(new)
    }

    fn sacrifice_and_return(&mut self, sacrificed: &[ObjectId], returned: &[(ObjectId, PlayerId)]) -> Result<(), Suspend> {
        ops::sacrifice_and_return(self, sacrificed, returned).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield_as(&mut self, objects: &[(ObjectId, PlayerId, bool)]) -> Result<Vec<ObjectId>, Suspend> {
        ops::put_onto_battlefield(self, objects, ZoneReason::Other).map_err(|_| Suspend)
    }

    fn put_all_onto_battlefield(&mut self, objects: &[(ObjectId, PlayerId)]) -> Result<Vec<ObjectId>, Suspend> {
        let all: Vec<(ObjectId, PlayerId, bool)> = objects.iter().map(|&(id, p)| (id, p, false)).collect();
        ops::put_onto_battlefield(self, &all, ZoneReason::Other).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_with_counters(&mut self, object: ObjectId, controller: PlayerId, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_with(self, &[(object, controller, tapped)], ZoneReason::Other, counters)
            .map(|v| v.first().copied().unwrap_or(object))
            .map_err(|_| Suspend)
    }

    fn create_tokens_with_counters(&mut self, def: CardId, controller: PlayerId, count: u32, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<Vec<ObjectId>, Suspend> {
        let tokens: Vec<(CardId, PlayerId)> = (0..count).map(|_| (def, controller)).collect();
        ops::create_tokens_batch(self, &tokens, tapped, None, counters).map_err(|_| Suspend)
    }

    fn create_token_copies(&mut self, of: ObjectId, controller: PlayerId, count: u32) -> Result<Vec<ObjectId>, Suspend> {
        let values = self.copiable_values(of);
        ops::create_token_copies(self, values, controller, count).map_err(|_| Suspend)
    }

    fn create_token_copies_of(&mut self, values: Copiable, controller: PlayerId, count: u32) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_token_copies(self, values, controller, count).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_transformed(&mut self, object: ObjectId, controller: PlayerId, tapped: bool) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_as(self, object, controller, tapped, true, ZoneReason::Other, &[]).map_err(|_| Suspend)
    }

    fn put_onto_battlefield_transformed_with_counters(&mut self, object: ObjectId, controller: PlayerId, tapped: bool, counters: &[(CounterKind, i16)]) -> Result<ObjectId, Suspend> {
        ops::put_onto_battlefield_as(self, object, controller, tapped, true, ZoneReason::Other, counters).map_err(|_| Suspend)
    }

    fn transform(&mut self, permanent: ObjectId) {
        ops::transform(self, permanent, None);
    }

    fn transform_by(&mut self, permanent: ObjectId, item: &StackItem) {
        ops::transform(self, permanent, Some(item));
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

    fn copy_ability(&mut self, ability: Target, copier: PlayerId, new_targets: bool) -> Result<Option<u32>, Suspend> {
        ops::copy_ability(self, ability, copier, new_targets).map_err(|_| Suspend)
    }

    fn delayed_trigger_about(&mut self, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, objects: &[ObjectId]) {
        let members = self.g.state.members_push(objects);
        let since = self.g.log.len() as u32;
        self.g.state.delayed_push(cantrip_state::Delayed { when, source, controller, ability, subject: Target::None, members, since });
    }

    fn delayed_trigger(&mut self, when: DelayedWhen, source: ObjectId, controller: PlayerId, ability: AbilityRef, subject: Target) {
        let since = self.g.log.len() as u32;
        self.g.state.delayed_push(cantrip_state::Delayed { when, source, controller, ability, subject, members: cantrip_state::Span::default(), since });
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
        self.emit(Event::ChosenAtRandom { count: count as u16, index: index as u16 });
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
        // CR 101.2: a player who can't get experience counters gets none (Solemnity's "players can't get counters").
        if static_rules::player_counters_banned(&*self, self.g.cards, player, PlayerCounter::Experience) {
            return;
        }
        self.g.state.update_player(player, |p| p.experience = p.experience.saturating_add(amount.min(255) as u8));
    }

    fn give_poison(&mut self, player: PlayerId, amount: u32) {
        ops::give_poison(self, player, amount);
    }

    fn become_copy(&mut self, source: ObjectId, permanent: ObjectId, values: Copiable, duration: Duration) {
        ops::become_copy(self, source, permanent, values, duration);
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

    fn searched(&mut self, player: PlayerId, library: PlayerId) {
        self.emit(Event::Searched { player, library });
    }

    fn scried(&mut self, player: PlayerId, count: u16, looked: u16) {
        self.emit(Event::Scried { player, count, looked });
    }

    fn surveilled(&mut self, player: PlayerId, count: u16, looked: u16) {
        self.emit(Event::Surveilled { player, count, looked });
    }

    fn attach(&mut self, attachment: ObjectId, host: ObjectId) {
        ops::attach(self, attachment, host);
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
        ops::monstrosity(self, permanent, counters);
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

    fn create_tokens_attacking(&mut self, tokens: &[(CardId, PlayerId)], defender: Target) -> Result<Vec<ObjectId>, Suspend> {
        ops::create_tokens_attacking(self, tokens, defender).map_err(|_| Suspend)
    }

    fn create_tokens_worded(&mut self, tokens: &[(CardId, PlayerId)], tapped: bool, attacking: Option<Target>, words: Words) -> Result<Vec<ObjectId>, Suspend> {
        let created = ops::create_tokens_worded(self, tokens, tapped || attacking.is_some(), None, &[], words, None).map_err(|_| Suspend)?;
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

    fn lose_life(&mut self, player: PlayerId, amount: i32) {
        ops::change_life(self, player, -amount);
    }

    fn set_life_total(&mut self, player: PlayerId, total: i32) {
        let life = self.g.state.player(player).life;
        ops::change_life(self, player, total - life);
    }

    fn add_mana(&mut self, player: PlayerId, mana: Mana, amount: u8) {
        let source = self.mana_source();
        self.g.state.add_mana(player, mana, amount, source, None, false);
    }

    fn become_day(&mut self) {
        day_night::become_designation(self, DayNight::Day);
    }

    fn become_night(&mut self) {
        day_night::become_designation(self, DayNight::Night);
    }

    fn note_done(&mut self, source: ObjectId, ability: AbilityRef) {
        let player = self.g.state.obj(source).controller;
        self.record(Event::Done { source, ability, player });
    }

    fn add_mana_until_end_of_combat(&mut self, player: PlayerId, mana: Mana, amount: u8) {
        let source = self.mana_source();
        self.g.state.add_mana(player, mana, amount, source, None, true);
    }

    fn add_restricted_mana(&mut self, player: PlayerId, mana: Mana, amount: u8, restriction: ManaRestriction) {
        let source = self.mana_source();
        self.g.state.add_mana(player, mana, amount, source, Some(restriction), false);
    }

    fn add_counters(&mut self, object: ObjectId, kind: CounterKind, amount: i16) {
        ops::add_counters(self, object, kind, amount);
    }

    fn add_effect(&mut self, source: ObjectId, affected: ObjectId, kind: EffectKind, duration: Duration) {
        // An object that has changed zones is a new object the effect knows nothing of (CR 400.7, 611.2c), and "for as long
        // as [the source] remains on the battlefield" is over before it begins once the source has left (CR 611.2b).
        if !(live(&self.g.state, affected) || kind.outlives_affected()) {
            return;
        }
        // A phased-out permanent is in no set of objects an effect affects, even one naming it (CR 702.26e).
        if self.g.state.obj(affected).zone.is(ZoneKind::PhasedOut) && !kind.outlives_affected() {
            return;
        }
        if duration == Duration::WhileSourceOnBattlefield
            && !(live(&self.g.state, source) && self.g.state.obj(source).zone.is(ZoneKind::Battlefield))
        {
            return;
        }
        // "For as long as it has a [kind] counter on it" whose condition is already false does nothing (CR 611.2b).
        if !duration.holds_for(self.g.state.obj(affected)) {
            return;
        }
        let controller = self.g.state.obj(source).controller;
        let timestamp = self.g.state.timestamp();
        // "For as long as [condition]" whose condition is false as it would begin does nothing (CR 611.2b).
        if let Duration::AsLongAs(..) = duration
            && !ops::lasting_holds(self, &ContinuousEffect { source, controller, timestamp, affected, kind, duration })
        {
            return;
        }
        self.g.state.effect_push(ContinuousEffect { source, controller, timestamp, affected, kind, duration });
        // A static ability given to a permanent that affects other objects makes it a source of static abilities.
        if let EffectKind::AddAbility(ability) = kind
            && matches!(self.cards().ability(ability), AbilityDef::Static(s) if !s.own)
        {
            self.g.state.see_statics();
        }
        if matches!(kind, EffectKind::Control(_)) {
            ops::refresh_control(self, affected);
        }
    }

    fn add_player_effect(&mut self, controller: PlayerId, source: ObjectId, kind: EffectKind, duration: Duration) {
        debug_assert!(kind.outlives_affected(), "a player's effect is about the player, not an object");
        let timestamp = self.g.state.timestamp();
        self.g.state.effect_push(ContinuousEffect { source, controller, timestamp, affected: source, kind, duration });
    }

    fn counter(&mut self, spell: ObjectId) {
        resolve::counter(self, spell);
    }

    fn counter_to(&mut self, spell: ObjectId, to: CounterTo) {
        resolve::counter_to(self, spell, to);
    }

    fn change_controller(&mut self, spell: ObjectId, player: PlayerId) {
        ops::change_spell_controller(self, spell, player);
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

    fn could_cast_during_resolution(&self, card: ObjectId) -> bool {
        actions::could_cast_during_resolution(self, card)
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

    fn cast_ways_during_resolution(&self, card: ObjectId) -> Vec<CastWay> {
        actions::cast_ways_during_resolution(self, card)
    }

    fn cast_during_resolution_as(&mut self, card: ObjectId, terms: CastTerms) -> Result<bool, Suspend> {
        actions::cast_during_resolution(self, card, terms).map_err(|_| Suspend)
    }

    fn create_copy_in_exile(&mut self, of: ObjectId, player: PlayerId) -> ObjectId {
        ops::copy_card_into(self, of, player, Zone::exile(player))
    }

    fn allow_play(&mut self, card: ObjectId, player: PlayerId, grant: PlayGrant) {
        let turn = self.g.state.turn.number;
        self.g.state.permission_push(PlayPermission { card, player, grant, turn });
    }

    fn could_cast_transformed_during_resolution(&self, card: ObjectId) -> bool {
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
            self.record(Event::AbilityActivated { source: permanent, ability, player });
        }
    }

    fn change_energy(&mut self, player: PlayerId, delta: i32) {
        ops::change_energy(self, player, delta);
    }

    fn proliferate(&mut self, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
        ops::proliferate(self, player, source).map_err(|_| Suspend)
    }

    fn set_copy(&mut self, object: ObjectId, copy: Copiable) {
        self.g.state.update_obj(object, |o| o.copy = Some(copy));
        if self.cards().statics[copy.card.0 as usize] {
            self.g.state.see_statics();
        }
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
        ZoneKind::Battlefield | ZoneKind::PhasedOut => obj.face_down && obj.controller != player,
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
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..sources.len() as u8).collect()),
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
