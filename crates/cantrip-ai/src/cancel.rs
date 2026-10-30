//! The AI's cancel guard (docs/ai-player-plan.md, "Answers"): an action a seat gives up is not offered again while the
//! board it was given up on stands. A cast or an activation cancelled part way is undone (CR 733.1), so without the
//! guard the same position is offered the same action, and a policy that liked it chooses it, cancels it and chooses it
//! again.
//!
//! The board is the game's position less the loop watch, the RNG, the next timestamp and the append-only history
//! (`board_hash`). None of those is what a cancelled action changes: its undo leaves the board as it was. The hash is
//! the AI's, not the engine's: the engine records the position, and the AI decides what "the same board" is for it.
//!
//! Three guards, from narrowest to widest:
//!
//! - An action it cancels (a cast or an activation, at priority or while a spell or ability resolves) is not offered
//!   again while the board is the same. A staged cast can be cancelled at more than one request, so each board keeps
//!   its own cancelled cards, and one board does not replace another.
//! - A cast cancelled while an ability resolves is not offered by that same ability again until priority returns.
//!   Uldaros Theorix's copies add history across the undo, so the board can look different.
//! - An action whose payment was cancelled is not offered again until the turn changes or a land is played: the same
//!   unpayable equip, with no change to the mana, would otherwise be chosen, cancelled and chosen again.
//!
//! The network's `Payment` uses the guard around each answer, and the search keeps one per line and per seat, filtering
//! the requests it plays as the real game would (`crate::search`).

use std::hash::{BuildHasher, Hash, Hasher};

use cantrip_engine::*;

/// A hash of the board, leaving out what a cancelled action does not change: the loop watch, the RNG, the next
/// timestamp, and the append-only history a cast or activation adds without moving a permanent (`members`,
/// `left_stack`, `spill_targets` and `share_arena`). A cancelled action's undo leaves this board unchanged, so the
/// same card is filtered from the next request's choices and a new position offers it again.
///
/// foldhash with a fixed seed: the hash is taken several times a search step, and it is the same in every process. The
/// guard only compares it for equality, and it never leaves the process.
pub fn board_hash(state: &GameState) -> u64 {
    let mut h = foldhash::fast::FixedState::with_seed(0).build_hasher();
    (
        &state.players,
        &state.restricted_mana,
        &state.counter_stamps,
        &state.mana_from,
    )
        .hash(&mut h);
    state.objects.hash(&mut h);
    state.zones.hash(&mut h);
    state.stack.hash(&mut h);
    state.turn.hash(&mut h);
    state.effects.hash(&mut h);
    (&state.triggers, &state.delayed, &state.permissions).hash(&mut h);
    (&state.attackers, &state.blocks, &state.extra_turns).hash(&mut h);
    state.agenda.hash(&mut h);
    state.outcome.hash(&mut h);
    h.finish()
}

/// Whether a `ChooseCards` purpose is a cast: a card chosen for one of these is an action that can be cancelled, and
/// so the one a cancelled action filters out.
fn is_cast(purpose: Purpose) -> bool {
    matches!(
        purpose,
        Purpose::Cast
            | Purpose::CastFree
            | Purpose::CastCopyFree
            | Purpose::CastCopyFreeWithin(_)
            | Purpose::CastTransformedFree
            | Purpose::CastForMadness
            | Purpose::CastFoundOrHand
            | Purpose::CastOrBottom
            | Purpose::CastForMiracleCost(_)
    )
}

fn request_source(request: &Request) -> Option<ObjectId> {
    match &request.kind {
        RequestKind::ChooseCards { source, .. }
        | RequestKind::ChooseTarget { source, .. }
        | RequestKind::ChooseModes { source, .. }
        | RequestKind::PayCosts { source, .. }
        | RequestKind::UnlessPays { source, .. }
        | RequestKind::ChooseColors { source, .. }
        | RequestKind::ChooseName { source, .. }
        | RequestKind::ChooseSubtype { source, .. }
        | RequestKind::ChooseNumber { source, .. }
        | RequestKind::May { source, .. }
        | RequestKind::ChoosePlayer { source, .. }
        | RequestKind::TapForPower { source, .. }
        | RequestKind::CollectEvidence { source, .. }
        | RequestKind::Craft { source, .. } => Some(*source),
        _ => None,
    }
}

fn cancels(answer: &Answer) -> bool {
    match answer {
        Answer::Cancel => true,
        Answer::Act(a) => a.kind == ActionKind::CancelCasting,
        _ => false,
    }
}

/// A seat's cancelled actions and what it is in the middle of (docs/ai-player-plan.md, "Answers"). The network keeps
/// one around each answer (`crate::net::Payment`); the search keeps one per line and per seat (`crate::search`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CancelGuard {
    /// Each board a cancelled action was given up on, and the cards whose casts or activations were cancelled there. A
    /// staged cast can be cancelled at more than one request, so one board does not replace another.
    pub(crate) cancelled: Vec<(u64, Vec<&'static str>)>,
    /// The last action the seat began (a cast or an activation), its source, and the board before it: what a later
    /// cancel is of.
    pub(crate) pending: Option<(u64, Option<ObjectId>, &'static str)>,
    /// A cast cancelled while an ability resolves: the ability offering it, the cards given up, and the turn. The
    /// board can differ across the undo (Uldaros Theorix's copy of a card does), so this filters the offer until
    /// priority returns.
    pub(crate) resolution: Option<(ObjectId, Vec<&'static str>, u32)>,
    /// The card the seat last chose at priority to cast or to activate an ability of: what a cancel cancels.
    pub(crate) casting: Option<&'static str>,
    /// Actions an unpayable payment cancelled this turn, by card name: not offered again until the turn changes or
    /// a land is played.
    pub(crate) turn_cancelled: Option<(u32, Vec<&'static str>)>,
}

/// A guard with its boards named by place (`CancelGuard::canonical`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Canonical {
    cancelled: Vec<(usize, Vec<&'static str>)>,
    pending: Option<(Option<usize>, Option<ObjectId>, &'static str)>,
    resolution: Option<(ObjectId, Vec<&'static str>, u32)>,
    casting: Option<&'static str>,
    turn_cancelled: Option<(u32, Vec<&'static str>)>,
}

/// A cancel a search is about to play: what to record once the game has undone the action.
pub(crate) struct Cancelled {
    source: Option<ObjectId>,
    name: &'static str,
    payment: bool,
}

impl CancelGuard {
    /// Whether the guard has nothing to filter or roll: a request then needs no board hash.
    pub(crate) fn is_empty(&self) -> bool {
        self.cancelled.is_empty()
            && self.pending.is_none()
            && self.resolution.is_none()
            && self.turn_cancelled.is_none()
    }

    /// `request` with the actions cancelled on the game's present board filtered out, after the turn and priority
    /// rollovers that make the guard current for this request.
    pub fn filtered(&mut self, game: &Game, request: &Request) -> Request {
        if self.is_empty() {
            return request.clone();
        }
        self.filtered_on(game, board_hash(&game.state), request)
    }

    /// `filtered`, on a board the caller has already hashed.
    pub(crate) fn filtered_on(&mut self, game: &Game, board: u64, request: &Request) -> Request {
        if self
            .turn_cancelled
            .as_ref()
            .is_some_and(|(turn, _)| *turn != game.state.turn.number)
        {
            self.turn_cancelled = None;
        }
        if matches!(request.kind, RequestKind::Priority { .. }) {
            // An action begun on this board and a priority on it again was undone (a cost could not be paid, for
            // instance): it is not offered again here, even though no answer said "cancel".
            if matches!(self.pending, Some((hash, _, _)) if hash == board)
                && let Some((_, source, name)) = self.pending.take()
            {
                self.record_cancel(game, board, source, name, true);
            }
            self.resolution = None;
        } else if self
            .resolution
            .as_ref()
            .is_some_and(|(_, _, turn)| *turn != game.state.turn.number)
        {
            self.resolution = None;
        }
        self.filtered_at(game, request, board)
    }

    /// `request` with the actions cancelled on `board` filtered out, without changing the guard.
    pub(crate) fn filtered_at(&self, game: &Game, request: &Request, board: u64) -> Request {
        let board_names: &[&str] = self
            .cancelled
            .iter()
            .find(|(hash, _)| *hash == board)
            .map_or(&[], |(_, names)| names.as_slice());
        let resolution_names: &[&str] = match &self.resolution {
            Some((source, names, turn))
                if *turn == game.state.turn.number && request_source(request) == Some(*source) =>
            {
                names
            }
            _ => &[],
        };
        let turn_names: &[&str] = match &self.turn_cancelled {
            Some((turn, names)) if *turn == game.state.turn.number => names,
            _ => &[],
        };
        if board_names.is_empty() && resolution_names.is_empty() && turn_names.is_empty() {
            return request.clone();
        }
        let cancelled = |name: &str| {
            board_names.iter().any(|n| *n == name)
                || resolution_names.iter().any(|n| *n == name)
                || turn_names.iter().any(|n| *n == name)
        };
        let mut request = request.clone();
        match &mut request.kind {
            RequestKind::Priority { options } => {
                options.retain(|o| o.source.is_none_or(|source| !cancelled(game.name(source))));
            }
            RequestKind::ChooseCards {
                purpose,
                cards,
                rule,
                ..
            } if is_cast(*purpose) => {
                let keep = |id: &ObjectId| !cancelled(game.name(*id));
                let exact = matches!(rule.count, Count::Exactly(_));
                cards.retain(keep);
                if let Some(among) = &mut rule.among {
                    among.retain(keep);
                }
                // A choice that cannot be left empty is left as it was; the answer guard still refuses the action.
                if cards.is_empty() && exact {
                    return request;
                }
            }
            // An "unless pays" (ward, a tax) whose payment was given up is declined, not paid and given up again.
            RequestKind::UnlessPays {
                source, options, ..
            } if cancelled(game.name(*source)) => {
                options.retain(|o| o.kind == ActionKind::DeclineCost);
            }
            _ => {}
        }
        request
    }

    /// Notes the answer given to `request` by the real game, where `board` is the board before it.
    pub fn note(&mut self, game: &Game, board: u64, request: &Request, answer: &Answer) {
        self.note_cancel(game, board, request, answer);
        self.note_action(game, board, request, answer);
        self.note_choice(game, request, answer);
    }

    /// Notes `answer` before a search plays it, while the objects it names still exist. It returns the cancelled
    /// action to record after the undo, when the answer cancels: the board a cancel is recorded at is the one its undo
    /// returns to, which a later request in the line will see.
    pub(crate) fn before(
        &mut self,
        game: &Game,
        request: &Request,
        answer: &Answer,
    ) -> Option<Cancelled> {
        if !cancels(answer) {
            if Self::stores_action(request, answer) {
                self.note_action(game, board_hash(&game.state), request, answer);
            }
            self.note_choice(game, request, answer);
            return None;
        }
        let payment = matches!(answer, Answer::Act(a) if a.kind == ActionKind::CancelCasting);
        let pending = self.pending.take();
        let found = match pending {
            Some((_, source, name)) => Some((source, name)),
            None => self.cancelled_action(game, request),
        };
        self.note_choice(game, request, answer);
        let (source, name) = found?;
        Some(Cancelled {
            source,
            name,
            payment,
        })
    }

    /// Records what `before` prepared, at the board the search's line returned to.
    pub(crate) fn after(&mut self, game: &Game, cancelled: Option<Cancelled>) {
        if let Some(cancelled) = cancelled {
            let board_after = board_hash(&game.state);
            self.record_cancel(
                game,
                board_after,
                cancelled.source,
                cancelled.name,
                cancelled.payment,
            );
        }
    }

    /// Keeps `name`'s casts and activations out of the offers at the game's present board, as a cancel there would.
    /// For tests that need a guarded position without playing the cancel out.
    #[doc(hidden)]
    pub fn ban(&mut self, game: &Game, name: &'static str) {
        let board = board_hash(&game.state);
        self.record_cancel(game, board, None, name, false);
    }

    /// Moves the guard from the real game's boards to a world's (`boards`: each real board's hash and the same board's in
    /// the world, as `search::world_boards` pairs them): its cancellations, and the board an action in progress began
    /// on, which a take-back in the world returns to as the world's board.
    pub fn rekey(&mut self, boards: &[(u64, u64)]) {
        for &(from, to) in boards {
            if from == to {
                continue;
            }
            if let Some(at) = self.cancelled.iter().position(|(hash, _)| *hash == from) {
                let (_, names) = self.cancelled.remove(at);
                match self.cancelled.iter_mut().find(|(hash, _)| *hash == to) {
                    Some((_, held)) => {
                        for name in names {
                            if !held.contains(&name) {
                                held.push(name);
                            }
                        }
                    }
                    None => self.cancelled.push((to, names)),
                }
            }
        }
        if let Some((hash, _, _)) = self.pending.as_mut()
            && let Some(&(_, to)) = boards.iter().find(|(from, _)| *from == *hash)
        {
            *hash = to;
        }
    }

    /// The guard as it acts in a game whose boards a line can return to are `boards` (`search::world_boards`, one
    /// game's side): each board hash by its place there, and any other hash as `None`, a board the game cannot come
    /// back to, which the guard then never matches. Two guards with the same canonical form act the same in their
    /// games (`search::ReuseCheck`): a hash only ever meets the board the game is on.
    pub(crate) fn canonical(&self, boards: &[u64]) -> Canonical {
        let place = |hash: u64| boards.iter().position(|b| *b == hash);
        Canonical {
            cancelled: self
                .cancelled
                .iter()
                .filter_map(|(hash, names)| place(*hash).map(|at| (at, names.clone())))
                .collect(),
            pending: self.pending.map(|(hash, source, name)| (place(hash), source, name)),
            resolution: self.resolution.clone(),
            casting: self.casting,
            turn_cancelled: self.turn_cancelled.clone(),
        }
    }

    /// Whether an answer begins an action whose board a later cancel would need.
    fn stores_action(request: &Request, answer: &Answer) -> bool {
        match (&request.kind, answer) {
            (RequestKind::Priority { .. }, Answer::Act(a)) => {
                matches!(a.kind, ActionKind::CastSpell | ActionKind::ActivateAbility)
            }
            (RequestKind::ChooseCards { purpose, .. }, Answer::Cards(cards)) => {
                is_cast(*purpose) && !cards.is_empty()
            }
            _ => false,
        }
    }

    fn note_action(&mut self, game: &Game, board: u64, request: &Request, answer: &Answer) {
        let (source, name) = match (&request.kind, answer) {
            (RequestKind::Priority { .. }, Answer::Act(a))
                if matches!(a.kind, ActionKind::CastSpell | ActionKind::ActivateAbility) =>
            {
                (a.source, a.source.map(|source| game.name(source)))
            }
            (
                RequestKind::ChooseCards {
                    purpose, source, ..
                },
                Answer::Cards(cards),
            ) if is_cast(*purpose) => (Some(*source), cards.first().map(|&card| game.name(card))),
            _ => (None, None),
        };
        if let Some(name) = name {
            self.pending = Some((board, source, name));
        }
    }

    fn note_cancel(&mut self, game: &Game, board: u64, request: &Request, answer: &Answer) {
        if !cancels(answer) {
            return;
        }
        // The board before the action that was cancelled, not the board mid-cast (the spell is on the stack there).
        let pending = self.pending.take();
        let (board, source, name) = match pending {
            Some((board, source, name)) => (board, source, name),
            None => {
                let Some((source, name)) = self.cancelled_action(game, request) else {
                    return;
                };
                (board, source, name)
            }
        };
        self.record_cancel(
            game,
            board,
            source,
            name,
            matches!(answer, Answer::Act(a) if a.kind == ActionKind::CancelCasting),
        );
    }

    /// What a cancel cancels where nothing is pending: at priority the card chosen there, everywhere else the request's
    /// source (a copy being cast, a spell's target choice, or a payment's own source).
    fn cancelled_action(
        &self,
        game: &Game,
        request: &Request,
    ) -> Option<(Option<ObjectId>, &'static str)> {
        let source = request_source(request);
        let name = if matches!(request.kind, RequestKind::Priority { .. }) {
            self.casting
                .or_else(|| source.map(|source| game.name(source)))
        } else {
            source.map(|source| game.name(source)).or(self.casting)
        }?;
        Some((source, name))
    }

    fn note_choice(&mut self, game: &Game, request: &Request, answer: &Answer) {
        if matches!(answer, Answer::Act(a) if a.kind == ActionKind::PlayLand) {
            // A new land can pay what was not payable a moment ago.
            self.turn_cancelled = None;
        }
        if let RequestKind::Priority { .. } = request.kind {
            self.casting = match answer {
                Answer::Act(a)
                    if matches!(a.kind, ActionKind::CastSpell | ActionKind::ActivateAbility) =>
                {
                    a.source.map(|s| game.name(s))
                }
                _ => None,
            };
        }
    }

    /// Keeps a cancelled action out of the offers at its board, and a cancelled cast out of its ability's offers
    /// until priority returns. A payment cancelled for want of mana is kept out until the turn changes or a land is
    /// played.
    fn record_cancel(
        &mut self,
        game: &Game,
        board: u64,
        source: Option<ObjectId>,
        name: &'static str,
        payment: bool,
    ) {
        match self.cancelled.iter_mut().find(|(hash, _)| *hash == board) {
            Some((_, names)) => {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            None => self.cancelled.push((board, vec![name])),
        }
        if let Some(source) = source {
            let turn = game.state.turn.number;
            match &mut self.resolution {
                Some((held, names, held_turn)) if *held == source && *held_turn == turn => {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                _ => self.resolution = Some((source, vec![name], turn)),
            }
        }
        if payment {
            let turn = game.state.turn.number;
            match &mut self.turn_cancelled {
                Some((held, names)) if *held == turn => {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                _ => self.turn_cancelled = Some((turn, vec![name])),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> StackItem {
        StackItem {
            kind: StackKind::Spell,
            object: ObjectId(1),
            ability: AbilityRef {
                card: CardId(0),
                index: 0,
            },
            controller: PlayerId(0),
            inline_targets: [Target::None; 16],
            target_count: 0,
            inline_spec_of: [0; 16],
            x: 0,
            kicked: 0,
            counted: 0,
            paid: 0,
            way: 0,
            modes: 0,
            choice: 0,
            subject: Target::None,
            data: [0; 2],
            members: Default::default(),
            serial: 0,
            gift: None,
            shares: StackItem::NO_SHARES,
            spill: 0,
            exile_instead: ExileInstead::Never,
            flashback: false,
            attacking: false,
            since: 0,
            copied: false,
            cast_known: u32::MAX,
            acquired: Acquired::OWN,
        }
    }

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

    /// The AI's cancel guard hashes the board, not the append-only history a cast or an activation adds: a cancelled
    /// action's undo can leave that history behind without changing what is on the board.
    #[test]
    fn board_hash_leaves_out_append_only_history() {
        let mut s = GameState::new(2, 1);
        let before = board_hash(&s);
        s.members.push(ObjectId(1));
        s.left_stack.push(item());
        s.unsettled = true;
        assert_eq!(board_hash(&s), before, "history is not the board");
        s.players[0].life -= 1;
        assert_ne!(board_hash(&s), before, "a changed player is the board");
    }

    /// A world's deal changes the hash of every board the guard names, not only the root's: the board an action in
    /// progress began on is moved too, so a take-back inside the world is recognised there.
    #[test]
    fn rekeying_moves_the_cancellations_and_the_board_an_action_began_on() {
        let mut guard = CancelGuard::default();
        guard.cancelled = vec![(1, vec!["Guard Test A"])];
        guard.pending = Some((2, None, "Guard Test B"));
        guard.rekey(&[(1, 10), (2, 20)]);
        assert_eq!(guard.cancelled, vec![(10, vec!["Guard Test A"])]);
        assert_eq!(guard.pending, Some((20, None, "Guard Test B")));
    }

    /// A cancel chosen inside a search's line is recorded at the board the undo returns to, not at the board the
    /// action was begun on: that is the board a later request in the line will see.
    #[test]
    fn a_cancel_in_a_line_is_recorded_at_the_board_it_returns_to() {
        let (mut game, a, b) = game();
        let before = board_hash(&game.state);
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
        let mut guard = CancelGuard::default();
        guard.pending = Some((before, Some(a), "Guard Test A"));
        let cancelled = guard.before(&game, &request, &Answer::Cancel);
        // The undo returns to a board that differs from the one the action was begun on.
        game.state.players[0].life -= 1;
        let after = board_hash(&game.state);
        guard.after(&game, cancelled);
        let RequestKind::Priority { options } = guard.filtered_at(&game, &request, after).kind
        else {
            panic!("a priority")
        };
        assert_eq!(
            options.iter().filter_map(|o| o.source).collect::<Vec<_>>(),
            vec![b],
            "the action given up is filtered"
        );
        let RequestKind::Priority { options } = guard.filtered_at(&game, &request, before).kind
        else {
            panic!("a priority")
        };
        assert_eq!(
            options.len(),
            2,
            "the board it was begun on does not filter it"
        );
    }
}
