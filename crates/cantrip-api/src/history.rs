//! The game's event history (the C# engine's `IEventHistory`, CR 603.10 and every "this turn" or "this game"
//! question): every rules event since the game began, in order, each with its serial, the turn it happened in and what
//! caused it. Append-only: nothing behind the head is ever edited, so a question asked of the history before some
//! serial has one answer however often it is asked.
//!
//! It is the open-ended record one-off mechanics ask about the past with ("the second creature spell you cast this
//! turn", "a permanent you controlled left the battlefield this turn", "cards put into your graveyard from the
//! battlefield this turn"), so no such question needs a field of its own in the state. A record names objects and
//! players and copies nothing out of them: what an object was is its row, which is never deleted, and for one that
//! left the battlefield its last-known snapshot (CR 608.2h).
//!
//! **When an event is in the history.** An event is recorded after the triggers it causes have been found and after
//! any replacement of it has been applied: a trigger matching an event, and a replacement deciding whether to replace
//! one, both see the history as it was just before that event. So a question such as "is this the first card they
//! draw in their draw step?" reads the same from a triggered ability (Orcish Bowmasters) and from a replacement effect
//! (Hullbreacher), and the questions cards share are written once, as functions over `History`, in
//! `cantrip_mechanics::history`.

use cantrip_state::*;

use crate::event::{Event, EventMask};

/// A spell as it was cast (CR 601.2i): its characteristics on the stack, and the zone it came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cast {
    /// The spell's object on the stack.
    pub spell: ObjectId,
    pub player: PlayerId,
    /// Its card types as it became cast, which its last-known row (`Cast::chars`) holds too: kept here so the many
    /// questions about the types of earlier casts ("noncreature spells you've cast this turn") need no state.
    pub types: TypeSet,
    /// The zone it was cast from (CR 601.2a).
    pub from: ZoneKind,
    /// Which of the card's own options it was cast by (`ActionOption::index`): 0 its mana cost, n its nth printed way, or
    /// another face's way number.
    pub way: u8,
    /// What let it be cast from where it was, beyond the rules every player has (`ActionOption::granted_by`, CR 601.3): the
    /// permanent whose permission it was (Lurrus of the Dream-Den, Mystic Forge), the spell or ability that gave one
    /// (Snapcaster Mage's), or the card's own retrace. "Once during each of your turns" and "if you cast a spell this way"
    /// ask it, through `granted_by`. `Grant::NONE` where nothing did.
    pub by: Grant,
    /// What its card's `CardDef::cast_fact` said of the board as it became cast ("if you controlled a Faerie as you cast this
    /// spell"); 0 for a card with none. A copy of a spell is not cast (CR 707.10) and has no `Cast`, so `fact_as_cast` is 0
    /// for it.
    pub fact: u32,
}

impl Cast {
    /// What let it be cast from where it was (`by`), if anything beyond the rules did.
    pub fn granted_by(&self) -> Option<Grant> {
        self.by.some()
    }

    pub fn is(&self, t: CardType) -> bool {
        self.types.has(t)
    }

    /// The spell's mana value as it became cast (CR 202.3e: X counted, 601.2i), from its last-known row.
    pub fn value(&self, q: &dyn crate::Query) -> u32 {
        let s = q.state();
        let item = s.stack.iter().rev().find(|i| i.kind == StackKind::Spell && i.object == self.spell).or_else(|| s.left_stack(self.spell));
        match item.and_then(|i| s.last_known.get(i.cast_known as usize)) {
            Some(row) => row.mana_value,
            None => q.mana_value(self.spell),
        }
    }

    /// The spell's characteristics as it became cast (CR 601.2i): "the first non-Lemur creature spell with flying you
    /// cast" asks of an earlier cast what it was then, not what an effect made it on the stack afterwards or what its card
    /// is once it has left.
    /// The row is the one its stack item names (`StackItem::cast_known`), on the stack or left it.
    pub fn chars(&self, q: &dyn crate::Query) -> crate::Chars {
        let s = q.state();
        let item = s.stack.iter().rev().find(|i| i.kind == StackKind::Spell && i.object == self.spell).or_else(|| s.left_stack(self.spell));
        let c = q.chars(self.spell);
        match item.and_then(|i| s.last_known.get(i.cast_known as usize)) {
            Some(row) => c.as_known(row),
            None => c,
        }
    }
}

/// A permanent as it entered the battlefield (CR 603.6a), as `History::entered` gives it: its `Event::Entered`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entered {
    pub permanent: ObjectId,
    /// Who controlled it as it entered.
    pub controller: PlayerId,
    /// Its card types as it entered, with the replacement effects and the battlefield's continuous effects applied (CR 614.12).
    pub types: TypeSet,
}

impl Entered {
    pub fn is(&self, t: CardType) -> bool {
        self.types.has(t)
    }
}

/// A creature declared as an attacker (CR 508.1; not one put onto the battlefield attacking, CR 508.4): its
/// `Event::Attacks`, as `History::attacked_with` gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Attacked {
    pub attacker: ObjectId,
    /// The last-known row that keeps it as it was declared (`Event::Attacks::known`).
    pub known: u32,
}

impl Attacked {
    /// Its characteristics as it was declared (CR 508.1): "you attacked with a Rogue" asks what it was then, whatever it
    /// has become, wherever it has gone and whoever controls it since. The rest - its card - is this object's.
    pub fn chars(&self, q: &dyn crate::Query) -> crate::Chars {
        let c = q.chars(self.attacker);
        let row = q.state().last_known.get(self.known as usize);
        debug_assert!(row.is_some_and(|r| r.id == self.attacker), "an attack names its own last-known row");
        match row {
            Some(row) => c.as_known(row),
            None => c,
        }
    }
}

/// Damage a source dealt (CR 120.3), as `History::damage_dealt` gives it: its `Event::Damage`, and the last-known row that
/// keeps the source as it was when it dealt it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DamageDealt {
    pub source: ObjectId,
    pub target: Target,
    pub amount: u16,
    pub combat: bool,
    /// Its controller as it dealt the damage (CR 120.3, 609.7).
    pub controller: PlayerId,
    pub known: u32,
}

impl DamageDealt {
    /// The source's characteristics as it dealt the damage: "a creature that, at the time it dealt that damage, was an
    /// Assassin creature" (freerunning, CR 702.173a) asks what it was then, whatever it has become, wherever it has gone and
    /// whoever controls it since (the ruling). The rest - its card - is this object's.
    pub fn chars(&self, q: &dyn crate::Query) -> crate::Chars {
        let c = q.chars(self.source);
        match q.state().last_known.get(self.known as usize).filter(|row| row.id == self.source) {
            Some(row) => c.as_known(row),
            None => c,
        }
    }
}

/// What made an event happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cause {
    /// The rules themselves: a state-based action, a trigger put on the stack, the game's setup.
    Rules,
    /// A turn-based action (CR 703): drawing for the turn, untapping, combat damage.
    Turn,
    /// A player's own action or the costs of it (CR 601, 602, 305).
    Player(PlayerId),
    /// A spell resolving.
    Spell(ObjectId),
    /// An activated or triggered ability resolving.
    Ability { source: ObjectId, ability: AbilityRef },
}

/// One thing that happened, as the history keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    /// Its place in the history: dense, from 0, never reused.
    pub serial: u32,
    /// The turn it happened in, the step of that turn, and whose turn it was.
    pub turn: u32,
    pub step: Step,
    pub active: PlayerId,
    pub cause: Cause,
    pub event: Event,
}

/// A question about the history: every condition that is set must hold.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventQuery {
    /// The kinds of event; none means any kind.
    pub kinds: EventMask,
    /// The player the event is about (`Event::player`).
    pub player: Option<PlayerId>,
    /// The object the event is about (`Event::object`): a zone change matches its old or its new object.
    pub object: Option<ObjectId>,
    /// For a zone change, why it happened.
    pub reason: Option<ZoneReason>,
    /// For a zone change, the kind of zone it left.
    pub from: Option<ZoneKind>,
    /// For a zone change, the kind of zone it went to.
    pub to: Option<ZoneKind>,
    /// For a life change, whether it was a gain (true) or a loss (false).
    pub gained: Option<bool>,
    /// Only this turn's events.
    pub this_turn: bool,
    /// Only events during this step (of whichever turn the other conditions allow).
    pub step: Option<Step>,
    /// Only events during this player's turns.
    pub in_turn_of: Option<PlayerId>,
    /// Only events after this serial.
    pub after: Option<u32>,
}

impl EventQuery {
    pub fn kind(kinds: EventMask) -> EventQuery {
        EventQuery { kinds, ..EventQuery::default() }
    }

    pub fn this_turn(self) -> EventQuery {
        EventQuery { this_turn: true, ..self }
    }

    pub fn player(self, player: PlayerId) -> EventQuery {
        EventQuery { player: Some(player), ..self }
    }

    pub fn object(self, object: ObjectId) -> EventQuery {
        EventQuery { object: Some(object), ..self }
    }

    pub fn reason(self, reason: ZoneReason) -> EventQuery {
        EventQuery { reason: Some(reason), ..self }
    }

    pub fn from(self, from: ZoneKind) -> EventQuery {
        EventQuery { from: Some(from), ..self }
    }

    pub fn to(self, to: ZoneKind) -> EventQuery {
        EventQuery { to: Some(to), ..self }
    }

    pub fn gained(self, gained: bool) -> EventQuery {
        EventQuery { gained: Some(gained), ..self }
    }

    pub fn step(self, step: Step) -> EventQuery {
        EventQuery { step: Some(step), ..self }
    }

    pub fn in_turn_of(self, player: PlayerId) -> EventQuery {
        EventQuery { in_turn_of: Some(player), ..self }
    }

    pub fn after(self, serial: u32) -> EventQuery {
        EventQuery { after: Some(serial), ..self }
    }

    fn matches(&self, r: &Record) -> bool {
        let e = &r.event;
        (self.kinds.0 == 0 || EventMask::of_history(e).intersects(self.kinds))
            && self.player.is_none_or(|p| e.player() == Some(p))
            && self.object.is_none_or(|o| e.is_about(o))
            && self.after.is_none_or(|s| r.serial > s)
            && self.step.is_none_or(|s| r.step == s)
            && self.in_turn_of.is_none_or(|p| r.active == p)
            && match *e {
                Event::ZoneChange { from, to, reason, .. } => {
                    self.reason.is_none_or(|x| x == reason)
                        && self.from.is_none_or(|x| from.is(x))
                        && self.to.is_none_or(|x| to.is(x))
                }
                _ => self.reason.is_none() && self.from.is_none() && self.to.is_none(),
            }
            && match *e {
                Event::LifeChange { delta, .. } => self.gained.is_none_or(|g| g == (delta > 0)),
                _ => self.gained.is_none(),
            }
    }
}

thread_local! {
    /// How many times the history has been read since `take_reads` was last asked (`History::new`, `note_read`).
    static READS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// How many times the history has been read on this thread, never started again (`reads_total`).
    static TOTAL: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times the history has been read on this thread: what the layers compare to know whether working something out
/// asked the past.
pub fn reads_total() -> u64 {
    TOTAL.with(|t| t.get())
}

/// The history is being read: a question about the past is being asked, whose answer may differ as the history grows.
/// Counted for the rules that find a loop of mandatory actions (CR 104.4b, `cantrip_rules::loops`): two positions alike in
/// everything but their history are the same position only if nothing asked the history in between. Every `History` is
/// made by `History::new`, which notes it; a rule that reads the records some other way notes it itself.
pub fn note_read() {
    READS.with(|r| r.set(r.get().wrapping_add(1)));
    TOTAL.with(|t| t.set(t.get().wrapping_add(1)));
}

/// How many times the history has been read on this thread since this was last asked, and starts the count again.
pub fn take_reads() -> u32 {
    READS.with(|r| r.replace(0))
}

/// The history as card code reads it: every record, and this turn's. Made by `History::new`, which notes the read.
#[derive(Clone, Copy)]
pub struct History<'a> {
    pub records: &'a [Record],
    /// The turn being played.
    pub turn: u32,
}

impl<'a> History<'a> {
    /// The history of a game, `records` its every record and `turn` the turn being played: a read of it (`note_read`).
    pub fn new(records: &'a [Record], turn: u32) -> History<'a> {
        note_read();
        History { records, turn }
    }

    /// Every record since the game began, oldest first.
    pub fn all(&self) -> &'a [Record] {
        self.records
    }

    /// This turn's records, oldest first: the tail of the history, found from the end.
    pub fn this_turn(&self) -> &'a [Record] {
        let start = self.records.iter().rposition(|r| r.turn != self.turn).map_or(0, |i| i + 1);
        &self.records[start..]
    }

    /// The records that answer a question, oldest first.
    pub fn select(&self, q: EventQuery) -> impl DoubleEndedIterator<Item = &'a Record> + 'a {
        let scope = if q.this_turn { self.this_turn() } else { self.records };
        scope.iter().filter(move |r| q.matches(r))
    }

    pub fn count(&self, q: EventQuery) -> usize {
        self.select(q).count()
    }

    pub fn any(&self, q: EventQuery) -> bool {
        self.select(q).next().is_some()
    }

    /// The serial the next record will have: an anchor for "since now".
    pub fn head(&self) -> u32 {
        self.records.len() as u32
    }

    // --- The questions cards ask most ---------------------------------------------------------------------------

    /// The spells cast this turn, in order, with their characteristics as cast.
    pub fn casts(&self) -> impl Iterator<Item = &'a Cast> + 'a {
        self.this_turn().iter().filter_map(|r| match &r.event {
            Event::Cast(c) => Some(c),
            _ => None,
        })
    }

    /// The spells `player` cast this turn.
    pub fn casts_by(&self, player: PlayerId) -> impl Iterator<Item = &'a Cast> + 'a {
        self.casts().filter(move |c| c.player == player)
    }

    /// The spells `player` has cast this game (CR 601.2i).
    pub fn casts_this_game(&self, player: PlayerId) -> usize {
        self.records.iter().filter(|r| matches!(&r.event, Event::Cast(c) if c.player == player)).count()
    }

    /// The permanents that entered the battlefield this turn, in order, as each was when it entered (CR 603.6a): the player who
    /// controlled it and its card types then (`Event::Entered`), whatever has happened to it since.
    pub fn entered(&self) -> impl Iterator<Item = Entered> + 'a {
        self.this_turn().iter().filter_map(|r| match r.event {
            Event::Entered { permanent, controller, types } => Some(Entered { permanent, controller, types }),
            _ => None,
        })
    }

    /// How the spell on the stack as `spell` was cast, if it was cast this turn.
    pub fn cast_of(&self, spell: ObjectId) -> Option<&'a Cast> {
        self.casts().find(|c| c.spell == spell)
    }

    /// What the board was as the spell on the stack as `spell` was cast, by its card's `CardDef::cast_fact` (CR 601.2i): 0 for
    /// a copy, which was not cast (CR 707.10). A spell is cast and resolves in the same turn (the stack is empty before a turn
    /// can end, and an effect that ends it exiles the stack, CR 724.1b), so this turn's casts are where it is.
    pub fn fact_as_cast(&self, spell: ObjectId) -> u32 {
        self.this_turn()
            .iter()
            .rev()
            .find_map(|r| match &r.event {
                Event::Cast(c) if c.spell == spell => Some(c.fact),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// The mana spent to cast the spell on the stack as `spell` (CR 601.2h), by the source that produced it (CR 106.3): each
    /// source with the mana of it spent, its total cost's whole (additional costs too), once for each part of the payment it
    /// gave to. Nothing for a copy, which was not cast and had no mana spent on it (CR 707.10), or for a spell cast without
    /// spending mana. A spell is cast and paid for in one turn, so this turn's records hold it.
    pub fn mana_spent_to_cast(&self, spell: ObjectId) -> impl Iterator<Item = (ObjectId, ManaTally)> + 'a {
        self.this_turn().iter().filter_map(move |r| match r.event {
            Event::ManaSpent { source, on, cast: true, mana, .. } if on == spell => Some((source, mana)),
            _ => None,
        })
    }

    /// Whether mana `source` produced was spent to cast `spell` (CR 106.3, 601.2h): "whenever you cast a spell using mana
    /// produced by [this]". `source` is the object as it produced the mana: one that has left and come back is another (CR
    /// 400.7).
    pub fn cast_using_mana_from(&self, spell: ObjectId, source: ObjectId) -> bool {
        self.mana_spent_to_cast(spell).any(|(from, _)| from == source)
    }

    /// The spell `spell` is a copy of, through a copy of a copy, or `spell` itself if it is no copy (CR 707.10): an effect of a
    /// copy that refers to the objects used to pay its costs uses those of the original spell. Fling and Kazuul's Fury
    /// (`cantrip_mechanics::history::sacrificed_creatures_power`) and "creatures that convoked it" (`convoked`) ask it.
    pub fn original_spell(&self, mut spell: ObjectId) -> ObjectId {
        while let Some(original) = self.records.iter().rev().find_map(|r| match r.event {
            Event::Copied { copy, original, .. } if copy == spell && original != spell => Some(original),
            _ => None,
        }) {
            spell = original;
        }
        spell
    }

    /// The creatures that convoked `object` (CR 702.51c), in the order they were tapped: `object` is a spell on the stack, or
    /// the permanent a spell became as it resolved (CR 608.3), which "each creature that convoked it" names. A copy of a
    /// spell was not cast, but its effects that refer to objects used to pay its costs use those of the original (CR 707.10):
    /// a copy, and the token a copy of a permanent spell became (CR 707.10f), read the original's - a copy of a copy too. A
    /// creature counts whether or not it is still on the battlefield (Knight-Errant of Eos's ruling); what it is, it is now
    /// or as it last existed (CR 608.2h). Nothing for an object that was never a spell, or a spell no creature convoked.
    /// Venerated Loxodon, Zephyr Singer, Knight-Errant of Eos, Ancient Imperiosaur, Lethal Scheme and Everything Comes to
    /// Dust ask it.
    pub fn convoked(&self, object: ObjectId) -> impl Iterator<Item = ObjectId> + 'a {
        let records = self.records;
        // The latest move that made `object`, if any: the move to the stack as it was cast (CR 601.2a), or, for a
        // permanent, the move from the stack as its spell resolved, whose `old` is the spell.
        let made = |object: ObjectId| {
            records.iter().rposition(|r| matches!(r.event, Event::ZoneChange { new, .. } if new == object))
        };
        let spell = match made(object).map(|at| records[at].event) {
            Some(Event::ZoneChange { old, from, to, .. }) if from.is(ZoneKind::Stack) && to.is(ZoneKind::Battlefield) => old,
            _ => object,
        };
        let spell = self.original_spell(spell);
        // Nothing before the spell was put on the stack names it (an object's id is never reused).
        let from = made(spell).unwrap_or(0);
        records[from..].iter().filter_map(move |r| match r.event {
            Event::Convoked { creature, spell: s } if s == spell => Some(creature),
            _ => None,
        })
    }

    /// How many times this ability of this object has resolved this turn (CR 608.2m).
    pub fn resolutions(&self, source: ObjectId, ability: AbilityRef) -> usize {
        self.this_turn()
            .iter()
            .filter(|r| matches!(r.event, Event::Resolved { source: s, ability: a } if s == source && a == ability))
            .count()
    }

    /// How many times this ability of this object has triggered this turn (CR 603.2h).
    pub fn triggerings(&self, source: ObjectId, ability: AbilityRef) -> usize {
        self.this_turn()
            .iter()
            .filter(|r| matches!(r.event, Event::Triggered { source: s, ability: a } if s == source && a == ability))
            .count()
    }

    /// Whether `player` has done this turn what this ability of this object says may be done only once each turn (CR
    /// 603.2h: it "triggers only if its source's controller has not yet taken the indicated action that turn"). Asked of
    /// the source's controller now, so a player who gains control of it after another did it may still do it once.
    pub fn done(&self, source: ObjectId, ability: AbilityRef, player: PlayerId) -> bool {
        self.this_turn().iter().any(|r| {
            matches!(r.event, Event::Done { source: s, ability: a, player: p } if s == source && a == ability && p == player)
        })
    }

    /// Cards `player` has drawn this turn (CR 121).
    pub fn drawn(&self, player: PlayerId) -> usize {
        self.count(EventQuery::kind(EventMask::DREW).player(player).this_turn())
    }

    /// Life `player` has gained this turn (CR 119.3).
    pub fn life_gained(&self, player: PlayerId) -> u32 {
        self.life(player, true)
    }

    /// Life `player` has lost this turn (CR 119.3).
    pub fn life_lost(&self, player: PlayerId) -> u32 {
        self.life(player, false)
    }

    fn life(&self, player: PlayerId, gained: bool) -> u32 {
        self.select(EventQuery::kind(EventMask::LIFE_CHANGE).player(player).gained(gained).this_turn())
            .map(|r| match r.event {
                Event::LifeChange { delta, .. } => delta.unsigned_abs(),
                _ => 0,
            })
            .sum()
    }

    /// Whether `object` was put into a graveyard by a discard this turn (CR 701.9): "if you discarded it this turn".
    pub fn discarded_this_turn(&self, object: ObjectId) -> bool {
        self.this_turn().iter().any(|r| {
            matches!(r.event, Event::ZoneChange { new, to, reason: ZoneReason::Discard, .. } if new == object && to.is(ZoneKind::Graveyard))
        })
    }

    /// The declarations of attackers `player` made this turn (CR 508.1; not a creature put onto the battlefield attacking,
    /// CR 508.4), in order, a creature that attacked in two combats twice: only the active player declares attackers (CR
    /// 506.2, 508.1), so none for another, whoever controls those creatures now. `Attacked::chars` is one as it was
    /// declared.
    pub fn attacked_with(&self, player: PlayerId) -> impl Iterator<Item = Attacked> + 'a {
        self.this_turn().iter().filter_map(move |r| match r.event {
            Event::Attacks { attacker, known, .. } if r.active == player => Some(Attacked { attacker, known }),
            _ => None,
        })
    }

    /// The damage dealt this turn, in order, each source's to each recipient (CR 120.3).
    pub fn damage_dealt(&self) -> impl Iterator<Item = DamageDealt> + 'a {
        self.this_turn().iter().filter_map(|r| match r.event {
            Event::Damage { source, target, amount, kind, controller, known, .. } => {
                Some(DamageDealt { source, target, amount, combat: kind.is_combat(), controller, known })
            }
            _ => None,
        })
    }

    /// Whether `creature` was declared as an attacker this turn (CR 508.1; not one put onto the battlefield attacking).
    pub fn attacked_this_turn(&self, creature: ObjectId) -> bool {
        self.this_turn().iter().any(|r| matches!(r.event, Event::Attacks { attacker, .. } if attacker == creature))
    }
}
