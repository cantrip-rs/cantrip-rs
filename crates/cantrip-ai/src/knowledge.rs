//! What one seat knows of the hidden zones, folded from the history: the hidden cards whose identity it knows, where in a
//! library it knows them to be, and how many of each card it has seen each opponent hold.
//!
//! Only the events say what a seat learnt: a reveal, a look, a question that showed it cards (`Event::Shown`), a card it
//! saw go into a hidden zone. A shuffle loses the order (`Event::Shuffled`), a random bottom the place
//! (`Event::Randomized`), a question that showed another player a card the place too (they may have moved it). The
//! seat follows a card out of a zone it cannot see only when it knows which one left: a library's card by its place.
//! Otherwise it can no longer tell which of its cards there is gone, and forgets them; a card it sees leave is taken to
//! be the one of that name it knew of. So what it knows depends on what it saw, never on which of two alike cards the
//! engine moved: two games it cannot tell apart give it the same knowledge. What it could look into when a card moved is
//! what it could then (`Event::PlayerControlled`: a hand is seen by the player controlling its player, CR 723.4), never
//! what it can now, so a fold record by record and one from scratch agree.
//!
//! The record holds engine ids, for the observation to look objects up by. It is never part of an observation.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use cantrip_engine::visibility::{knows_face_as, looks_into_as};
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
    /// The players another player controls, and who (CR 723.1), as of the records folded.
    controllers: BTreeMap<PlayerId, PlayerId>,
}

impl Knowledge {
    /// What `seat` knows at the deal: nothing hidden.
    pub fn new(seat: PlayerId) -> Knowledge {
        Knowledge { seat, folded: 0, last: None, known: BTreeMap::new(), traced: BTreeSet::new(), seen: BTreeMap::new(), floating: BTreeMap::new(), controllers: BTreeMap::new() }
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
        // A fold from scratch is as long as the history: checked each time the history passes another 256 records. Not
        // in a search's fork (`Game::fork_for_search`): its hidden cards were dealt anew at its root (`rebind_hidden`), and a
        // fold from scratch reads the present state, so it would take a past event to be of the card dealt there since
        // (a card drawn taken for a copy the seat knew of in its library, which the world has since made another card).
        // What the seat knows there is what it knew of the real game, folded on.
        if verify() && continues && game.tree.root() == tree::NodeId::ROOT && (before / 256 != self.folded / 256) {
            let fresh = Knowledge::of_scratch(game, self.seat);
            assert!(
                *self == fresh,
                "seat {}'s knowledge folded record by record differs from a fold from scratch:
  known {:?}
   vs   {:?}
  traced {:?}
   vs    {:?}
  seen {:?}
   vs  {:?}
  floating {:?}
   vs      {:?}",
                self.seat.0,
                self.known.iter().filter(|(k, v)| fresh.known.get(k) != Some(v)).collect::<Vec<_>>(),
                fresh.known.iter().filter(|(k, v)| self.known.get(k) != Some(v)).collect::<Vec<_>>(),
                self.traced.difference(&fresh.traced).collect::<Vec<_>>(),
                fresh.traced.difference(&self.traced).collect::<Vec<_>>(),
                self.seen.iter().filter(|(k, v)| fresh.seen.get(k) != Some(v)).collect::<Vec<_>>(),
                fresh.seen.iter().filter(|(k, v)| self.seen.get(k) != Some(v)).collect::<Vec<_>>(),
                self.floating,
                fresh.floating,
            );
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

    /// Whether the seat saw what `player` saw as of the records folded (CR 723.4: `GameState::sees_as` then).
    fn sees_as(&self, player: PlayerId) -> bool {
        player == self.seat || self.controllers.get(&player) == Some(&self.seat)
    }

    fn looks_into(&self, zone: Zone) -> bool {
        looks_into_as(self.seat, zone, |p| self.sees_as(p))
    }

    fn knows_face(&self, game: &Game, id: ObjectId) -> bool {
        knows_face_as(&game.state, id, |p| self.sees_as(p))
    }

    fn fold(&mut self, game: &Game, event: &Event) {
        let s = &game.state;
        match *event {
            Event::Revealed { card, .. } => self.learn(game, card),
            // An ability activated from a hand or a library reveals its card (CR 602.2a: ninjutsu, cycling), which the
            // history records as the activation itself.
            Event::AbilityActivated { source, .. } if matches!(s.obj(source).zone.kind(), ZoneKind::Hand | ZoneKind::Library) => {
                self.learn(game, source)
            }
            // What a player looks at, the player controlling them sees too (CR 723.4).
            Event::Looked { player, card } | Event::Shown { player, card } if self.sees_as(player) => self.learn(game, card),
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
            Event::PlayerControlled { player, controller } if controller == player => {
                self.controllers.remove(&player);
            }
            Event::PlayerControlled { player, controller } => {
                self.controllers.insert(player, controller);
            }
            _ => {}
        }
    }

    /// The seat was shown this card: it knows it where it is hidden, and has seen it.
    fn learn(&mut self, game: &Game, id: ObjectId) {
        let zone = game.state.obj(id).zone;
        if !self.looks_into(zone) && zone.kind().is_hidden() {
            self.known.insert(id, zone.is(ZoneKind::Library));
        }
        self.see(game, id);
    }

    fn moved(&mut self, game: &Game, old: ObjectId, new: ObjectId, from: Zone, to: Zone) {
        let s = &game.state;
        let stored = self.known.remove(&old);
        let traced = self.traced.remove(&old);
        let sees_from = self.looks_into(from) && !from.is(ZoneKind::Gone);
        let sees_to = self.looks_into(to);
        let sees_new = sees_to && self.knows_face(game, new);
        // It can tell which object left: it sees the zone, or knows the library card's place.
        let follows = sees_from || from.is(ZoneKind::Gone) || stored == Some(true);
        let identity = follows && (stored.is_some() || (sees_from && self.knows_face(game, old)));
        if follows {
            if identity && !sees_to && to.kind().is_hidden() {
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
