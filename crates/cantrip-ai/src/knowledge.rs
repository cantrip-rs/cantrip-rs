//! What one seat knows of the hidden zones, folded from the history: the hidden cards whose identity it knows, where in a
//! library it knows them to be, and how many of each card it has seen each opponent hold.
//!
//! Only the events say what a seat learnt: a reveal, a look, a question that showed it cards (`Event::Shown`), a card it
//! saw go into a hidden zone. A shuffle loses the order (`Event::Shuffled`), a random bottom the place
//! (`Event::Randomized`), a question that showed another player a card the place too (they may have moved it). The
//! seat follows a card out of a zone it cannot see only when it knows which one left: a library's card by its place.
//! Otherwise it can no longer tell which of its cards there is gone, and forgets them; a card it sees leave is taken to
//! be the one of that name it knew of. So what it knows depends on what it saw, never on which of two alike cards the
//! engine moved: two games it cannot tell apart give it the same knowledge.
//!
//! The record holds engine ids, for the observation to look objects up by. It is never part of an observation.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use cantrip_engine::visibility::{knows_face, looks_into};
use cantrip_engine::*;

/// One seat's knowledge of a game, folded from its history up to `folded` records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Knowledge {
    seat: PlayerId,
    folded: usize,
    last: Option<Record>,
    /// Hidden objects whose card the seat knows, and whether it knows where in its library each is.
    known: BTreeMap<ObjectId, bool>,
    /// Opponents' cards the seat has counted as seen, while it can still follow them.
    traced: BTreeSet<ObjectId>,
    /// How many of each card (by its front) the seat has seen each opponent hold, and how many of those it lost track of.
    seen: BTreeMap<(PlayerId, CardId), u16>,
    floating: BTreeMap<(PlayerId, CardId), u16>,
}

impl Knowledge {
    /// What `seat` knows at the deal: nothing hidden.
    pub fn new(seat: PlayerId) -> Knowledge {
        Knowledge { seat, folded: 0, last: None, known: BTreeMap::new(), traced: BTreeSet::new(), seen: BTreeMap::new(), floating: BTreeMap::new() }
    }

    /// Knowledge of the game as it stands, folded from scratch.
    pub fn of(game: &Game, seat: PlayerId) -> Knowledge {
        let mut k = Knowledge::new(seat);
        k.update(game);
        k
    }

    pub fn seat(&self) -> PlayerId {
        self.seat
    }

    /// Folds the records added since the last update; from scratch if the history was cut back or went another way.
    /// Under `CANTRIP_VERIFY` it checks the result against a fold from scratch now and then.
    pub fn update(&mut self, game: &Game) {
        let log = &game.log;
        let continues = self.folded <= log.len() && (self.folded == 0 || log.get(self.folded - 1) == self.last.as_ref());
        if !continues {
            *self = Knowledge::new(self.seat);
        }
        let before = self.folded;
        for record in &log[self.folded..] {
            self.fold(game, &record.event);
        }
        self.folded = log.len();
        self.last = log.last().copied();
        // A fold from scratch is as long as the history: checked each time the history passes another 256 records.
        if verify() && continues && (before / 256 != self.folded / 256) {
            let fresh = Knowledge::of_scratch(game, self.seat);
            assert!(*self == fresh, "seat {}'s knowledge folded record by record differs from a fold from scratch", self.seat.0);
        }
    }

    fn of_scratch(game: &Game, seat: PlayerId) -> Knowledge {
        let mut k = Knowledge::new(seat);
        for record in &game.log {
            k.fold(game, &record.event);
        }
        k.folded = game.log.len();
        k.last = game.log.last().copied();
        k
    }

    /// Whether the seat knows what this hidden object is, and if so whether it knows its place in its library.
    pub fn knows(&self, id: ObjectId) -> Option<bool> {
        self.known.get(&id).copied()
    }

    /// The hidden objects the seat knows the card of.
    pub fn known(&self) -> impl Iterator<Item = (ObjectId, bool)> + '_ {
        self.known.iter().map(|(&id, &placed)| (id, placed))
    }

    /// How many of each card (by its front) the seat has seen `opponent` hold this game, wherever they are now.
    pub fn seen(&self, opponent: PlayerId) -> impl Iterator<Item = (CardId, u16)> + '_ {
        self.seen.iter().filter(move |((p, _), _)| *p == opponent).map(|(&(_, card), &n)| (card, n))
    }

    fn fold(&mut self, game: &Game, event: &Event) {
        let s = &game.state;
        let me = self.seat;
        match *event {
            Event::Revealed { card, .. } => self.learn(game, card),
            Event::Looked { player, card } | Event::Shown { player, card } if player == me => self.learn(game, card),
            // Another player looked at it, and may have put it anywhere among what they looked at.
            Event::Looked { card, .. } | Event::Shown { card, .. } => {
                if let Some(placed) = self.known.get_mut(&card) {
                    *placed = false;
                }
            }
            Event::Randomized { card, .. } => {
                if let Some(placed) = self.known.get_mut(&card) {
                    *placed = false;
                }
            }
            Event::Shuffled { player } => {
                let library = Zone::library(player);
                for (&id, placed) in self.known.iter_mut() {
                    if s.obj(id).zone == library {
                        *placed = false;
                    }
                }
            }
            Event::TurnedFaceUp { permanent, .. } => self.see(game, permanent),
            Event::ZoneChange { old, new, from, to, .. } => self.moved(game, old, new, from, to),
            _ => {}
        }
    }

    /// The seat was shown this card: it knows it where it is hidden, and has seen it.
    fn learn(&mut self, game: &Game, id: ObjectId) {
        let zone = game.state.obj(id).zone;
        if !looks_into(self.seat, zone) && zone.kind().is_hidden() {
            self.known.insert(id, zone.is(ZoneKind::Library));
        }
        self.see(game, id);
    }

    fn moved(&mut self, game: &Game, old: ObjectId, new: ObjectId, from: Zone, to: Zone) {
        let s = &game.state;
        let me = self.seat;
        let stored = self.known.remove(&old);
        let traced = self.traced.remove(&old);
        let sees_from = looks_into(me, from) && !from.is(ZoneKind::Gone);
        let sees_new = looks_into(me, to) && knows_face(s, me, new);
        // It can tell which object left: it sees the zone, or knows the library card's place.
        let follows = sees_from || from.is(ZoneKind::Gone) || stored == Some(true);
        let identity = follows && (stored.is_some() || (sees_from && knows_face(s, me, old)));
        if follows {
            if identity && !looks_into(me, to) && to.kind().is_hidden() {
                // Put on top or at the bottom is a place it knows; shuffled in or at random, the events that follow say not.
                self.known.insert(new, to.is(ZoneKind::Library));
            }
            if traced {
                self.traced.insert(new);
            } else if sees_new {
                self.see(game, new);
            }
            return;
        }
        // One of the cards of a zone it cannot see left, and it cannot tell which.
        let card = front(game, s.obj(old).card);
        if sees_new {
            // It sees what left: that is the one of that name it knew of there, if it knew of one.
            if stored.is_none() {
                let other = self.known.iter().find(|&(&id, &placed)| !placed && s.obj(id).zone == from && front(game, s.obj(id).card) == card).map(|(&id, _)| id);
                if let Some(id) = other {
                    self.known.remove(&id);
                }
            }
            let other = (!traced).then(|| self.traced.iter().copied().find(|&id| s.obj(id).zone == from && front(game, s.obj(id).card) == card)).flatten();
            match other {
                Some(id) => {
                    self.traced.remove(&id);
                    self.traced.insert(new);
                }
                None if traced => {
                    self.traced.insert(new);
                }
                None => self.see(game, new),
            }
        } else {
            // Any of those it knew of there may be the one that left.
            let lost: Vec<ObjectId> = self.known.iter().filter(|&(&id, &placed)| !placed && s.obj(id).zone == from).map(|(&id, _)| id).collect();
            for id in lost {
                self.known.remove(&id);
            }
            let lost: Vec<ObjectId> = self.traced.iter().copied().filter(|&id| s.obj(id).zone == from && self.known.get(&id) != Some(&true)).collect();
            for id in lost.into_iter().chain(traced.then_some(old)) {
                self.traced.remove(&id);
                let o = s.obj(id);
                *self.floating.entry((o.owner, front(game, o.card))).or_default() += 1;
            }
        }
    }

    /// The seat sees an opponent's card: counted once for as long as it can follow it, and a card it lost track of
    /// may be the one it sees again.
    fn see(&mut self, game: &Game, id: ObjectId) {
        let o = game.state.obj(id);
        if o.owner == self.seat || o.token || o.card_copy || self.traced.contains(&id) {
            return;
        }
        self.traced.insert(id);
        let key = (o.owner, front(game, o.card));
        match self.floating.get_mut(&key) {
            Some(n) if *n > 0 => *n -= 1,
            _ => *self.seen.entry(key).or_default() += 1,
        }
    }
}

/// A card by its front face: either face of a double-faced card names it.
pub(crate) fn front(game: &Game, card: CardId) -> CardId {
    game.cards.fronts.get(card.0 as usize).copied().flatten().unwrap_or(card)
}

fn verify() -> bool {
    static VERIFY: OnceLock<bool> = OnceLock::new();
    *VERIFY.get_or_init(|| std::env::var_os("CANTRIP_VERIFY").is_some_and(|v| v != "0"))
}
