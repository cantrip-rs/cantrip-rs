//! What a seat may see (CR 400.2, 401.2, 402.3, 406.3, 708.5): the one rule the host's views and the AI's observations
//! both follow, so neither can see more than the other.
//!
//! A seat sees the zones it may look into, every spell and ability on the stack, and what is being shown to it right now
//! (`extra`: a reveal, a look, the objects its own outstanding request names). A face-down object's face goes to its
//! controller on the battlefield and to those who saw it go there in exile. While the asked player is part way through
//! announcing or paying, the log past the frame's start can still be cut back; it is theirs alone until then
//! (`deferred`).

use std::collections::HashSet;

use crate::*;

/// The key an ability on the stack has in a handle space: abilities are no objects of the engine, but a client points
/// at one (to target it, to order it), so each is named by the serial it was put on the stack with.
pub fn ability_key(serial: u32) -> ObjectId {
    ObjectId(0x8000_0000 | serial)
}

pub fn is_ability_key(id: ObjectId) -> bool {
    id.0 & 0x8000_0000 != 0
}

/// Whether `viewer` may look into a zone.
pub fn looks_into(viewer: PlayerId, zone: Zone) -> bool {
    match zone.kind() {
        // A player may look at their own sideboard at any time (CR 400.11a; North Wind Avatar's ruling); nobody else may.
        ZoneKind::Hand | ZoneKind::Sideboard => zone.player == viewer,
        ZoneKind::Library | ZoneKind::Gone => false,
        _ => true,
    }
}

/// One entry of the stack as a viewer sees it.
#[derive(Clone, Copy, Debug)]
pub enum StackEntry {
    /// A spell (its object), and the item announced for it once it is paid for.
    Spell { id: ObjectId, item: Option<StackItem> },
    /// An activated or triggered ability, named by its serial.
    Ability { item: StackItem },
}

impl StackEntry {
    /// What a handle space keys it by.
    pub fn key(&self) -> ObjectId {
        match self {
            StackEntry::Spell { id, .. } => *id,
            StackEntry::Ability { item } => ability_key(item.serial),
        }
    }
}

/// The stack as the rules have it, bottom first: its items, then a spell still being announced (on the stack by its
/// zone, with no item yet).
pub fn stack_entries(state: &GameState) -> Vec<StackEntry> {
    let mut out: Vec<StackEntry> = Vec::new();
    for item in &state.stack {
        match item.kind {
            StackKind::Spell => out.push(StackEntry::Spell { id: item.object, item: Some(*item) }),
            _ => out.push(StackEntry::Ability { item: *item }),
        }
    }
    for &id in state.zone(Zone::STACK) {
        if !out.iter().any(|e| matches!(e, StackEntry::Spell { id: s, .. } if *s == id)) {
            out.push(StackEntry::Spell { id, item: None });
        }
    }
    out
}

/// Every object this viewer can observe: the contents of the zones they may look into, every spell and ability on the
/// stack, and what is being shown to them right now (`extra`).
pub fn observable(game: &Game, viewer: PlayerId, extra: &HashSet<ObjectId>) -> HashSet<ObjectId> {
    let s = &game.state;
    let mut visible = HashSet::new();
    for p in s.seats() {
        if p == viewer {
            visible.extend(s.zone(Zone::hand(p)).iter().copied());
            visible.extend(s.zone(Zone::sideboard(p)).iter().copied());
        }
        visible.extend(s.zone(Zone::graveyard(p)).iter().copied());
        visible.extend(s.zone(Zone::exile(p)).iter().copied());
    }
    visible.extend(s.battlefield().iter().copied());
    visible.extend(s.zone(Zone::PHASED_OUT).iter().copied());
    visible.extend(s.zone(Zone::COMMAND).iter().copied());
    // "You may look at the top card of your library any time" (CR 401.2): its player sees it while a permanent says so.
    if game.may_look_at_library_top(viewer)
        && let Some(&top) = s.zone(Zone::library(viewer)).first()
    {
        visible.insert(top);
    }
    visible.extend(stack_entries(s).iter().map(StackEntry::key));
    for &id in extra {
        if !is_ability_key(id) && (id.index() < s.objects.len()) && live(s, id) {
            visible.insert(id);
        }
    }
    visible
}

/// Whether `viewer` may see what a face-down object really is (CR 708.5, 406.3): its controller on the battlefield,
/// those who saw it go there anywhere else. True of an object that is face up.
pub fn knows_face(state: &GameState, viewer: PlayerId, id: ObjectId) -> bool {
    let obj = state.obj(id);
    if !obj.face_down {
        return true;
    }
    if obj.zone.is(ZoneKind::Battlefield) || obj.zone.is(ZoneKind::PhasedOut) {
        obj.controller == viewer
    } else {
        obj.seen_by & (1 << viewer.0) != 0
    }
}

/// The objects a request names, which its player must be able to see for as long as it is outstanding: a library being
/// searched, the cards looked at, an opponent's hand taken from. They are shown to that seat alone.
pub fn request_objects(request: &Request) -> HashSet<ObjectId> {
    let mut out = HashSet::new();
    request.kind.objects(&mut |id| {
        out.insert(id);
    });
    out
}

/// The log up to where it can no longer be cut back: the start of the frame waiting on an answer, while one is.
pub fn publishable(game: &Game) -> usize {
    let len = game.log.len();
    match game.state.agenda.last() {
        Some(frame) if frame.pending.is_some() => (frame.log_mark as usize).min(len),
        _ => len,
    }
}

/// Whether what `seat` sees must wait: another player is part way through a run (announcing, paying) whose log can
/// still be cut back, so that seat keeps what it was last shown.
pub fn deferred(game: &Game, seat: PlayerId) -> bool {
    publishable(game) < game.log.len() && !game.is_over() && game.pending().is_some_and(|r| r.player != seat)
}
