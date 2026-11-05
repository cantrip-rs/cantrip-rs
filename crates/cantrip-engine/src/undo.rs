//! Taking back a mana ability: a courtesy of the table, not a rule, so it is offered only while nothing hidden has been
//! shown and nothing has followed from it. While the last answer a player gave activated a mana ability (CR 605.1a),
//! and it and everything since only tapped, sacrificed from the battlefield, removed counters, added mana or cost life
//! - a land, Black Lotus, Lotus Petal, Pentad Prism, a Treasure - the player may take it back, and the game goes back to
//! before it through the game-log tree (`Game::goto`). Never taken back: an ability whose cost discards (Lion's Eye
//! Diamond shows the hand), and one that triggered anything (Chromatic Star draws). Any other answer, casting a spell
//! included, clears what can be taken back.
//!
//! The host and the scenario runner share it: what one offers, the other can script (`Bot1.Undo`).

use cantrip_api::*;
use cantrip_rules::Game;
use cantrip_rules::tree::NodeId;

/// The mana abilities that can still be taken back, latest last, each with what its keeper holds beside it (the host's
/// told events and handles; nothing for the scenario runner).
pub struct Undo<T> {
    entries: Vec<Entry<T>>,
    pending: Option<(Before, T, usize, usize)>,
}

struct Entry<T> {
    player: PlayerId,
    source: ObjectId,
    node: NodeId,
    kept: T,
}

/// An answer about to be given, as taking it back needs it remembered.
pub struct Before {
    player: PlayerId,
    source: Option<ObjectId>,
    node: NodeId,
    log_from: usize,
    triggers: usize,
    stack: usize,
    priority_depth: Option<usize>,
    cancels: bool,
}

impl Before {
    /// Whether the answer activates a mana ability that could be taken back, for a keeper to know whether to save
    /// what it would need.
    pub fn is_mana(&self) -> bool {
        self.source.is_some()
    }
}

impl<T> Default for Undo<T> {
    fn default() -> Self {
        Undo { entries: Vec::new(), pending: None }
    }
}

impl<T> Undo<T> {
    /// Notes `answer`, about to be given by `player`.
    pub fn before(game: &Game, player: PlayerId, answer: &Answer) -> Before {
        Before {
            player,
            source: mana_activation(game, answer),
            node: game.checkpoint(),
            log_from: game.log.len(),
            triggers: game.state.triggers.len(),
            stack: game.state.stack.len(),
            priority_depth: matches!(game.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. })).then_some(game.state.agenda.len()),
            cancels: matches!(answer, Answer::Act(action) if action.kind == ActionKind::CancelCasting),
        }
    }

    /// Whether the answer `before` noted, now given, may be taken back. If not, nothing before it may be either.
    pub fn may_take_back(&mut self, game: &Game, before: &Before) -> bool {
        let asks_again = game.pending().is_some_and(|next| {
            next.player == before.player && matches!(next.kind, RequestKind::Priority { .. } | RequestKind::PayCosts { .. })
        });
        // A trigger is something the ability did (Chromatic Star's draw), waiting or on the stack, even before anyone
        // has seen what it does.
        let undoable = before.source.is_some()
            && asks_again
            && only_mana(&game.log[before.log_from.min(game.log.len())..])
            && game.state.triggers.len() <= before.triggers
            && game.state.stack.len() <= before.stack;
        if !undoable {
            self.entries.clear();
        }
        undoable
    }

    /// Follows a priority-started mana activation across its suspended cost and effect questions. Only public payment
    /// and color questions are retained; all other questions conservatively end the courtesy undo history.
    pub fn track(&mut self, game: &Game, before: Before, kept: Option<T>) -> bool {
        if let Some((start, saved, depth, previously_activated)) = self.pending.take() {
            let active = game.state.agenda.get(depth).is_some_and(|f| {
                matches!(f.kind, cantrip_state::FrameKind::Act { player } if player == start.player)
            });
            if active {
                let safe = game.pending().is_some_and(|r| r.player == start.player && matches!(r.kind,
                    RequestKind::PayCosts { .. } | RequestKind::ChooseCostPayment { .. } | RequestKind::ChooseColors { .. }));
                if safe { let count = activation_count(game, &start); self.pending = Some((start, saved, depth, count)); return true; }
                self.entries.clear();
                return false;
            }
            let completed = !before.cancels && activation_count(game, &start) > previously_activated;
            if completed && self.may_take_back(game, &start) { self.push(start, saved); return true; }
            self.entries.clear();
            return false;
        }
        if let Some(saved) = kept {
            let depth = before.priority_depth.unwrap_or(usize::MAX);
            let active = game.state.agenda.get(depth).is_some_and(|f| {
                matches!(f.kind, cantrip_state::FrameKind::Act { player } if player == before.player)
            });
            if active {
                let safe = game.pending().is_some_and(|r| r.player == before.player && matches!(r.kind,
                    RequestKind::PayCosts { .. } | RequestKind::ChooseCostPayment { .. } | RequestKind::ChooseColors { .. }));
                if safe { let count = activation_count(game, &before); self.pending = Some((before, saved, depth, count)); return true; }
                self.entries.clear();
                return false;
            }
            if self.may_take_back(game, &before) { self.push(before, saved); return true; }
            return false;
        }
        if self.may_take_back(game, &before) { /* No retained snapshot means no undo entry. */ }
        false
    }

    /// Records the answer `before` noted as one that may be taken back, with what the keeper holds beside it. Call
    /// only when [`Undo::may_take_back`] said so.
    pub fn push(&mut self, before: Before, kept: T) {
        let source = before.source.expect("a mana ability");
        self.entries.push(Entry { player: before.player, source, node: before.node, kept });
    }

    /// Whether this player has an activation being followed through its payment questions.
    pub fn pending_for(&self, player: PlayerId) -> bool {
        self.pending.as_ref().is_some_and(|(before, _, _, _)| before.player == player)
    }

    /// The permanent whose mana ability `player` may take back now, if any.
    pub fn offered(&self, player: PlayerId) -> Option<ObjectId> {
        if self.pending.is_some() { None } else { self.entries.last().filter(|e| e.player == player).map(|e| e.source) }
    }

    /// What the keeper holds beside the latest entry.
    pub fn last_kept(&mut self) -> Option<&mut T> {
        if let Some((_, kept, _, _)) = &mut self.pending { Some(kept) } else { self.entries.last_mut().map(|e| &mut e.kept) }
    }

    /// Takes back the latest mana ability `player` activated: the game goes back to the node before it, and what the
    /// keeper held beside it is returned. None if `player` has nothing to take back.
    pub fn take_back(&mut self, game: &mut Game, player: PlayerId) -> Option<T> {
        self.offered(player)?;
        let entry = self.entries.pop()?;
        game.goto(entry.node);
        Some(entry.kept)
    }

    /// Nothing may be taken back any more: a concession, or the game set aside.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pending = None;
    }
}

/// Replayed activation records for this source since the original priority answer. Requiring a new record on the
/// final answer avoids treating an earlier nested activation of the same source as a completed outer activation.
fn activation_count(game: &Game, before: &Before) -> usize {
    game.log[before.log_from.min(game.log.len())..].iter().filter(|r| matches!(r.event,
        Event::AbilityActivated { source, player, .. } if Some(source) == before.source && player == before.player)).count()
}

/// The source of a mana ability `answer` activates (CR 605.1a), if it activates one that could be taken back: not one
/// whose cost discards, which shows what was in the hand.
fn mana_activation(game: &Game, answer: &Answer) -> Option<ObjectId> {
    let Answer::Act(action) = answer else { return None };
    let source = action.source.filter(|_| action.kind == ActionKind::ActivateAbility)?;
    if action.index >= PLAYER_MANA_ABILITY {
        return Some(source);
    }
    match game.chars(source).abilities().nth(action.index as usize).map(|a| game.cards.ability(a)) {
        Some(AbilityDef::Activated(d)) if d.is_mana_ability() && !d.cost.discard_hand && !d.cost.discard_self && d.cost.discard.is_none() => {
            Some(source)
        }
        _ => None,
    }
}

/// Whether these entries are only what a mana ability does: taps, sacrifices from the battlefield, mana made and spent, and life paid
/// or lost for it.
fn only_mana(records: &[Record]) -> bool {
    records.iter().all(|r| match r.event {
        Event::ZoneChange { from, reason, .. } => reason == ZoneReason::Sacrifice && from.kind() == ZoneKind::Battlefield,
        Event::Tapped { .. }
        | Event::TappedForMana { .. }
        | Event::ManaSpent { .. }
        | Event::LifeChange { .. }
        | Event::AbilityActivated { .. }
        | Event::Resolved { .. }
        | Event::Damage { target: Target::Player(_), .. } => true,
        _ => false,
    })
}
