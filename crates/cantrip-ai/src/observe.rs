//! One seat's observation of a game (`cantrip_obs::Observation`): what the shared visibility lets it see, what its
//! knowledge says of the hidden cards, and nothing else.
//!
//! Objects are numbered canonically: by place (each seat's library, hand, graveyard, exile and sideboard, then the
//! battlefield, phased-out permanents, the stack and the command zone), a library, a graveyard and the stack by
//! position, any other zone by what is visible of each object, refs left out. Where two objects look alike the order
//! they came into the zone breaks the tie, which every player saw. No engine id is in an observation.

use std::collections::{BTreeMap, HashMap, HashSet};

use cantrip_engine::visibility::{StackEntry, ability_key, deferred, knows_face, looks_into, observable, request_objects, stack_entries};
use cantrip_engine::*;
use cantrip_obs::*;
use cantrip_obs::Count;

use crate::knowledge::{Knowledge, front};

/// How many of the latest events an observation carries.
pub const EVENTS: usize = 32;

/// `seat`'s observation of the game as it stands, in `mode`. `None` while what the seat sees must wait (another player
/// is part way through announcing or paying, and the log past their frame can still be cut back: `deferred`): the
/// caller keeps the last observation it had, as the host keeps a seat's last view.
pub fn observe(game: &Game, knowledge: &Knowledge, seat: PlayerId, mode: InfoMode) -> Option<Observation> {
    observe_indexed(game, knowledge, seat, mode).map(|(observation, _)| observation)
}

/// The observation and, by canonical index, the engine id each object is (an ability by its `ability_key`): for the
/// player that answers, to turn an answer by index back into the engine's, and for tests. Never part of an observation.
pub fn observe_indexed(game: &Game, knowledge: &Knowledge, seat: PlayerId, mode: InfoMode) -> Option<(Observation, Vec<ObjectId>)> {
    if deferred(game, seat) {
        return None;
    }
    debug_assert_eq!(knowledge.seat(), seat, "an observation is made from the seat's own knowledge");
    let s = &game.state;
    let request = game.pending();
    let extra = match request {
        Some(r) if cantrip_engine::visibility::answering(game, r) == seat => request_objects(r),
        _ => HashSet::new(),
    };
    let view = View { game, knowledge, seat, full: mode == InfoMode::Full, visible: observable(game, seat, &extra) };

    // Each place's objects in canonical order, built once with every ref left out to sort by.
    let mut slots: Vec<Slot> = Vec::new();
    let none = HashMap::new();
    for p in s.seats() {
        for zone in [Zone::library(p), Zone::hand(p), Zone::graveyard(p), Zone::exile(p), Zone::sideboard(p)] {
            view.place(zone, &none, &mut slots);
        }
    }
    for zone in [Zone::BATTLEFIELD, Zone::PHASED_OUT] {
        view.place(zone, &none, &mut slots);
    }
    for (i, entry) in stack_entries(s).into_iter().enumerate() {
        slots.push(Slot { id: entry.key(), stack: Some((i, entry)), obs: view.stacked(i, &entry, &none) });
    }
    view.place(Zone::COMMAND, &none, &mut slots);

    // The refs: to what the seat sees, by index; anything else is out of its sight.
    let refs: HashMap<ObjectId, u32> = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| !matches!(slot.obs, ObjectObs::Hidden { .. }))
        .map(|(i, slot)| (slot.id, i as u32))
        .collect();
    // Each object as sorted, with its refs filled in: nothing else of it depends on them.
    let index: Vec<ObjectId> = slots.iter().map(|slot| slot.id).collect();
    let objects: Vec<ObjectObs> = slots
        .into_iter()
        .map(|mut slot| {
            match (&mut slot.obs, slot.stack) {
                (ObjectObs::Hidden { .. }, _) => {}
                (ObjectObs::Visible(v), _) => view.refer(slot.id, v, &refs),
                (ObjectObs::Ability { source, targets, .. }, Some((_, StackEntry::Ability { item }))) => {
                    (*source, *targets) = view.ability_refs(&item, &refs);
                }
                (ObjectObs::Ability { .. }, _) => unreachable!("an ability is only on the stack"),
            }
            slot.obs
        })
        .collect();

    let observation = Observation {
        schema: SCHEMA_VERSION,
        mode,
        seat: seat.0,
        turn: TurnObs { number: s.turn.number, active: s.turn.active.0, step: s.turn.step_raw, priority: s.turn.priority.0, starting: s.turn.starting.0 },
        players: s.seats().map(|p| view.player(p)).collect(),
        objects,
        events: view.events(&refs),
        request: request.map(|r| RequestObs {
            player: r.player.0,
            kind: kind_name(&r.kind),
            candidates: if cantrip_engine::visibility::answering(game, r) == seat {
                let mut c: Vec<u32> = request_objects(r).iter().filter_map(|id| refs.get(id).copied()).collect();
                c.sort_unstable();
                c.dedup();
                c
            } else {
                Vec::new()
            },
            ask: (r.player == seat).then(|| crate::ask::ask_of(game, r, &refs).0),
        }),
        decklist: counts(view.roots(seat).map(|id| view.card_name(id))),
        pools: s.seats().map(|p| if p != seat && mode == InfoMode::Decklist { view.pool(p) } else { Vec::new() }).collect(),
        seen: s
            .seats()
            .map(|p| {
                let mut seen: Vec<Count> =
                    if p == seat { Vec::new() } else { knowledge.seen(p).map(|(card, count)| Count { name: game.cards.get(card).name.to_string(), count }).collect() };
                seen.sort();
                seen
            })
            .collect(),
    };
    Some((observation, index))
}

/// An object in canonical order, as first built with its refs left out.
struct Slot {
    id: ObjectId,
    stack: Option<(usize, StackEntry)>,
    obs: ObjectObs,
}

struct View<'a> {
    game: &'a Game,
    knowledge: &'a Knowledge,
    seat: PlayerId,
    full: bool,
    visible: HashSet<ObjectId>,
}

impl View<'_> {
    fn sees(&self, id: ObjectId) -> bool {
        self.full || self.visible.contains(&id)
    }

    fn card_name(&self, id: ObjectId) -> String {
        self.game.cards.get(front(self.game, self.game.state.obj(id).card)).name.to_string()
    }

    /// A zone's objects, in canonical order: a library and a graveyard as they lie, any other zone sorted by what the
    /// seat sees of each.
    fn place(&self, zone: Zone, refs: &HashMap<ObjectId, u32>, out: &mut Vec<Slot>) {
        let s = &self.game.state;
        let ordered = zone.is(ZoneKind::Library) || zone.is(ZoneKind::Graveyard);
        let place = place_of(zone);
        let mut slots: Vec<(Vec<u8>, Slot)> = s
            .zone(zone)
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let position = ordered.then_some(i as u16);
                let obs = if self.sees(id) { ObjectObs::Visible(Box::new(self.visible(id, place, position, refs))) } else { self.hidden(id, place, i) };
                let key = if ordered { Vec::new() } else { cantrip_obs_bytes(&obs) };
                (key, Slot { id, stack: None, obs })
            })
            .collect();
        if !ordered {
            slots.sort_by(|a, b| a.0.cmp(&b.0));
        }
        out.extend(slots.into_iter().map(|(_, slot)| slot));
    }

    fn hidden(&self, id: ObjectId, place: Place, i: usize) -> ObjectObs {
        let o = self.game.state.obj(id);
        let library = o.zone.is(ZoneKind::Library);
        // A library's card by name only where the seat knows its place; elsewhere it counts in `library_known`.
        let identity = match self.knowledge.knows(id) {
            Some(placed) if placed || !library => Some(self.game.cards.get(o.card).name.to_string()),
            _ => None,
        };
        ObjectObs::Hidden { place, owner: o.owner.0, position: library.then_some(i as u16), identity, face_down: o.face_down }
    }

    fn visible(&self, id: ObjectId, place: Place, position: Option<u16>, refs: &HashMap<ObjectId, u32>) -> VisibleObs {
        let game = self.game;
        let s = &game.state;
        let o = s.obj(id);
        let on_battlefield = o.zone.is(ZoneKind::Battlefield) || o.zone.is(ZoneKind::PhasedOut);
        let looks = self.full || knows_face(s, self.seat, id);
        // Face down off the battlefield and the stack it has no characteristics at all (CR 406.3); a face-down permanent or
        // spell has its face-down values (CR 708.2a, 708.4).
        let chars = if o.face_down && !on_battlefield && !o.zone.is(ZoneKind::Stack) {
            CharsObs { types: 0, supertypes: 0, colors: 0, subtypes: Vec::new(), power: None, toughness: None, keywords: 0, mana_value: 0 }
        } else {
            let c = game.chars(id);
            CharsObs {
                types: c.types.0,
                supertypes: u8::from(c.supertypes.basic) | u8::from(c.supertypes.legendary) << 1 | u8::from(c.supertypes.snow) << 2,
                colors: c.colors.0,
                subtypes: c.subtypes.iter().map(|t| t as u16).collect(),
                power: c.power,
                toughness: c.toughness,
                keywords: c.keywords.0,
                mana_value: Query::mana_value(game, id),
            }
        };
        let name = if o.face_down { looks.then(|| game.cards.get(o.card).name.to_string()) } else { Some(identity(game, id).to_string()) };
        let attacker = s.attackers.iter().find(|a| a.id == id);
        let item = s.stack.iter().find(|item| item.kind == StackKind::Spell && item.object == id);
        let mut v = VisibleObs {
            place,
            position,
            owner: o.owner.0,
            controller: o.controller.0,
            name,
            chars,
            face_down: o.face_down,
            token: o.token,
            tapped: o.tapped,
            damage: o.damage,
            counters: o.counter_array().iter().enumerate().filter(|&(_, &n)| n != 0).map(|(k, &n)| (k as u16, n)).collect(),
            // The refs are `refer`'s.
            attacking: None,
            blocked: attacker.is_some_and(|a| a.blocked),
            blocking: Vec::new(),
            sick: on_battlefield && summoning_sick(game.cards, s, &game.log, id),
            attached: Ref::None,
            spell: o.zone.is(ZoneKind::Stack).then(|| SpellObs {
                controller: item.map_or(o.controller, |i| i.controller).0,
                targets: Vec::new(),
                x: item.map_or(o.x, |i| i.x),
            }),
        };
        self.refer(id, &mut v, refs);
        v
    }

    /// The fields of a visible object that name other objects, by `refs`: all that `refs` changes of it, so an object
    /// first described with its refs left out (to sort by) needs only these again.
    fn refer(&self, id: ObjectId, v: &mut VisibleObs, refs: &HashMap<ObjectId, u32>) {
        let s = &self.game.state;
        let o = s.obj(id);
        let r = |t: Target| target_ref(t, refs);
        v.attacking = s.attackers.iter().find(|a| a.id == id).map(|a| r(a.target));
        v.blocking = s.blocks.iter().filter(|b| b.blocker == id).map(|b| r(Target::Object(b.attacker))).collect();
        v.attached = match (o.attached_to, o.attached_player) {
            (Some(host), _) => r(Target::Object(host)),
            (None, Some(p)) => Ref::Player(p.0),
            (None, None) => Ref::None,
        };
        if let Some(spell) = &mut v.spell {
            let item = s.stack.iter().find(|item| item.kind == StackKind::Spell && item.object == id);
            spell.targets = item.map_or_else(Vec::new, |i| s.targets(i).iter().map(|&t| r(t)).collect());
        }
    }

    /// The stack's `i`th entry from the bottom.
    fn stacked(&self, i: usize, entry: &StackEntry, refs: &HashMap<ObjectId, u32>) -> ObjectObs {
        let place = Place { zone: ZoneObs::Stack, player: 0 };
        match *entry {
            StackEntry::Spell { id, .. } => ObjectObs::Visible(Box::new(self.visible(id, place, Some(i as u16), refs))),
            StackEntry::Ability { item } => {
                let s = &self.game.state;
                let source = s.obj(item.object);
                // Named for its source where the seat could see that: a card it can see now, or last saw.
                let public = self.sees(item.object) || (!live(s, item.object) && (self.full || (looks_into(s, self.seat, source.zone) && knows_face(s, self.seat, item.object))));
                let name = if !public {
                    String::new()
                } else if live(s, item.object) && !source.face_down {
                    identity(self.game, item.object).to_string()
                } else {
                    self.game.cards.get(source.card).name.to_string()
                };
                let (source, targets) = self.ability_refs(&item, refs);
                ObjectObs::Ability {
                    position: i as u16,
                    source,
                    name,
                    kind: if item.kind == StackKind::Activated { 1 } else { 2 },
                    controller: item.controller.0,
                    targets,
                    x: item.x,
                }
            }
        }
    }

    /// A stacked ability's source and targets, by `refs`: all that `refs` changes of it.
    fn ability_refs(&self, item: &StackItem, refs: &HashMap<ObjectId, u32>) -> (Ref, Vec<Ref>) {
        (target_ref(Target::Object(item.object), refs), self.game.state.targets(item).iter().map(|&t| target_ref(t, refs)).collect())
    }

    fn player(&self, p: PlayerId) -> PlayerObs {
        let s = &self.game.state;
        let pl = &s.players[p.index()];
        let library = Zone::library(p);
        let mut library_known: Vec<String> = if self.full {
            Vec::new()
        } else {
            self.knowledge.known().filter(|&(id, placed)| !placed && s.obj(id).zone == library).map(|(id, _)| self.game.cards.get(s.obj(id).card).name.to_string()).collect()
        };
        library_known.sort();
        PlayerObs {
            life: pl.life,
            poison: pl.poison(),
            energy: pl.energy(),
            lost: pl.lost,
            lands_played: pl.lands_played,
            mulligans: pl.mulligans,
            pool: pl.pool.0,
            hand: s.zone(Zone::hand(p)).len() as u16,
            library: s.zone(library).len() as u16,
            monarch: pl.monarch,
            speed: pl.speed,
            library_known,
        }
    }

    /// A player's cards as dealt: those that began in their library, which no other object became.
    fn roots(&self, p: PlayerId) -> impl Iterator<Item = ObjectId> + '_ {
        let s = &self.game.state;
        let became: HashSet<ObjectId> = s.objects.iter().filter_map(|o| o.became).collect();
        let library = Zone::library(p);
        (0..s.objects.len() as u32).map(ObjectId).filter(move |&id| {
            let o = s.obj(id);
            o.zone == library && !o.token && !o.card_copy && !became.contains(&id)
        })
    }

    /// An opponent's decklist less their cards the seat sees or knows now.
    fn pool(&self, p: PlayerId) -> Vec<Count> {
        let s = &self.game.state;
        let mut pool: BTreeMap<String, i32> = BTreeMap::new();
        for id in self.roots(p) {
            *pool.entry(self.card_name(id)).or_default() += 1;
        }
        for id in (0..s.objects.len() as u32).map(ObjectId) {
            let o = s.obj(id);
            let known = self.knowledge.knows(id).is_some() || (self.visible.contains(&id) && knows_face(s, self.seat, id));
            if o.owner == p && !o.token && !o.card_copy && live(s, id) && known
                && let Some(n) = pool.get_mut(&self.card_name(id))
            {
                *n -= 1;
            }
        }
        pool.into_iter().filter(|&(_, n)| n > 0).map(|(name, n)| Count { name, count: n as u16 }).collect()
    }

    /// The latest public events. What only some player was shown (`Looked`, `Shown`) and where a card went at random
    /// are not among them.
    ///
    /// An event names an object only where the seat could have followed it from the event to now. An object keeps its
    /// id only while it stays in one zone, so in a zone the seat looks into it could; in a hidden one (another player's
    /// hand, a library) it could not, even while it sees the object for a moment (a hand revealed, a library searched):
    /// which of the cards it sees now is the one drawn two turns ago, or put there before a shuffle, it cannot tell.
    fn events(&self, refs: &HashMap<ObjectId, u32>) -> Vec<EventObs> {
        let s = &self.game.state;
        let followed: HashMap<ObjectId, u32> = refs
            .iter()
            .filter(|&(&id, _)| {
                if self.full || id.0 & 0x8000_0000 != 0 {
                    return true;
                }
                let zone = s.obj(id).zone;
                !zone.kind().is_hidden() || looks_into(s, self.seat, zone)
            })
            .map(|(&id, &i)| (id, i))
            .collect();
        let refs = &followed;
        let o = |id: ObjectId| target_ref(Target::Object(id), refs);
        let mut out: Vec<EventObs> = self
            .game
            .log
            .iter()
            .rev()
            .filter(|r| !matches!(r.event, Event::Looked { .. } | Event::Shown { .. } | Event::Randomized { .. }))
            .take(EVENTS)
            .map(|r| {
                let e = &r.event;
                let mut obs = EventObs {
                    turn: r.turn,
                    kind: EventMask::of_history(e).0.trailing_zeros() as u8,
                    player: e.player().map(|p| p.0),
                    objects: Vec::new(),
                    from: None,
                    to: None,
                    amount: 0,
                };
                match *e {
                    Event::ZoneChange { new, from, to, .. } => {
                        obs.objects = vec![o(new)];
                        obs.from = Some(place_of(from));
                        obs.to = Some(place_of(to));
                    }
                    Event::Damage { source, target, amount, .. } => {
                        obs.objects = vec![o(source), target_ref(target, refs)];
                        obs.amount = i32::from(amount);
                    }
                    Event::LifeChange { delta, .. } => obs.amount = delta,
                    Event::Drew { card, .. } | Event::Discarded { card, .. } | Event::Revealed { card, .. } | Event::Cycled { card, .. } => obs.objects = vec![o(card)],
                    Event::SpellCast { spell, .. } | Event::Countered { spell } => obs.objects = vec![o(spell)],
                    Event::Attacks { attacker, defender, .. } => obs.objects = vec![o(attacker), target_ref(defender, refs)],
                    Event::Blocks { blocker, attacker, .. } => obs.objects = vec![o(blocker), o(attacker)],
                    Event::Tapped { permanent } | Event::TurnedFaceUp { permanent, .. } | Event::Transformed { object: permanent } => obs.objects = vec![o(permanent)],
                    Event::CountersPut { object, kind, amount, .. } => {
                        obs.objects = vec![o(object)];
                        obs.amount = i32::from(amount) << 8 | kind as i32;
                    }
                    Event::AbilityActivated { source, .. } | Event::Triggered { source, .. } | Event::Resolved { source, .. } => obs.objects = vec![o(source)],
                    _ => {}
                }
                obs
            })
            .collect();
        out.reverse();
        out
    }
}

pub(crate) fn target_ref(target: Target, refs: &HashMap<ObjectId, u32>) -> Ref {
    let index = |id: ObjectId| refs.get(&id).map_or(Ref::None, |&i| Ref::Object(i));
    match target {
        Target::None => Ref::None,
        Target::Player(p) => Ref::Player(p.0),
        Target::Object(id) => index(id),
        Target::Ability(serial) => index(ability_key(serial)),
    }
}

fn place_of(zone: Zone) -> Place {
    let (zone_obs, player) = match zone.kind() {
        ZoneKind::Library => (ZoneObs::Library, zone.player.0),
        ZoneKind::Hand => (ZoneObs::Hand, zone.player.0),
        ZoneKind::Graveyard => (ZoneObs::Graveyard, zone.player.0),
        ZoneKind::Exile => (ZoneObs::Exile, zone.player.0),
        ZoneKind::Sideboard => (ZoneObs::Sideboard, zone.player.0),
        ZoneKind::Battlefield => (ZoneObs::Battlefield, 0),
        ZoneKind::PhasedOut => (ZoneObs::PhasedOut, 0),
        ZoneKind::Stack => (ZoneObs::Stack, 0),
        // Nothing observed is gone; a token that left the battlefield is out of the command zone's way.
        ZoneKind::Command | ZoneKind::Gone => (ZoneObs::Command, 0),
    };
    Place { zone: zone_obs, player }
}

/// The name of the card an object is: that of the card whose printed values its copiable values are (`Chars::card`), a
/// row of the card index. Not the name it shows, which need be no card's: a token the text that made it names for its
/// subtypes is named for them as a text change made them ("Phyrexian Elf Token" of a Hydra token, CR 111.4, 612.2a).
fn identity(game: &Game, id: ObjectId) -> &'static str {
    game.cards.get(game.chars(id).card).name
}

/// A request's kind by name: its variant.
pub fn kind_name(kind: &RequestKind) -> String {
    let debug = format!("{kind:?}");
    let end = debug.find(['{', '(', ' ']).unwrap_or(debug.len());
    debug[..end].to_string()
}

fn counts(names: impl Iterator<Item = String>) -> Vec<Count> {
    let mut by: BTreeMap<String, u16> = BTreeMap::new();
    for name in names {
        *by.entry(name).or_default() += 1;
    }
    by.into_iter().map(|(name, count)| Count { name, count }).collect()
}

fn cantrip_obs_bytes(obs: &ObjectObs) -> Vec<u8> {
    postcard::to_allocvec(obs).expect("an object always serializes")
}
