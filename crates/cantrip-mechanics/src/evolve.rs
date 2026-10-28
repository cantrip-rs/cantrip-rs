//! Evolve (CR 702.100a): "Whenever a creature you control enters, if that creature's power is greater than this creature's
//! power and/or that creature's toughness is greater than this creature's toughness, put a +1/+1 counter on this creature."
//! Each instance triggers separately (CR 702.100d). Adaptive Snapjaw, Battering Krasis, Clinging Anemones, Cloudfin Raptor,
//! Crocanura, Gluttonous Slug and Shambleshark print it.
//!
//! A card with evolve lists `EVOLVE` among its `abilities`.
//!
//! The intervening "if" (CR 603.4) is about the creature that entered, which a `TriggerCondition` is not told of, so it is
//! asked in both places by the same function: as the creature enters, by the trigger (compared with its power and toughness
//! as it entered, the counters it entered with included), and again first thing as the ability resolves, against its power
//! and toughness then - or as it last existed on the battlefield if it has left (the rulings).

use cantrip_api::*;

/// What evolve compares of a creature: whether it is one, and its power and toughness. Those it has now while it is on the
/// battlefield, and as it last existed there once it has left (CR 608.2h, 113.7a).
struct Stats {
    creature: bool,
    power: Option<i16>,
    toughness: Option<i16>,
}

fn stats(q: &dyn Query, id: ObjectId) -> Stats {
    if crate::targets::permanent(q, id) {
        let c = q.chars(id);
        Stats { creature: c.is(CardType::Creature), power: c.power, toughness: c.toughness }
    } else {
        let none = Stats { creature: false, power: None, toughness: None };
        q.state().last_known(id).map_or(none, |k| Stats { creature: k.types.has(CardType::Creature), power: k.power, toughness: k.toughness })
    }
}

/// "That creature's power is greater than this creature's power and/or that creature's toughness is greater than this
/// creature's toughness" (CR 702.100a): power against power and toughness against toughness (the rulings). A creature can't
/// have a greater power or toughness than a noncreature permanent (CR 702.100c), so a source that is not a creature has none
/// to be greater than, and a permanent that is not a creature has none greater than anything's.
fn is_greater(q: &dyn Query, this: ObjectId, entered: ObjectId) -> bool {
    let (this, entered) = (stats(q, this), stats(q, entered));
    if !this.creature || !entered.creature {
        return false;
    }
    let greater = |entered: Option<i16>, this: Option<i16>| matches!((entered, this), (Some(e), Some(t)) if e > t);
    greater(entered.power, this.power) || greater(entered.toughness, this.toughness)
}

/// "Whenever a creature you control enters, if that creature's power is greater ... " (CR 603.6a, 603.4): the creature that
/// entered is the ability's subject.
fn greater_creature_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && is_greater(q, me.source, *new) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Put a +1/+1 counter on this creature" (CR 122.1), if the intervening "if" still holds (CR 603.4): the comparison is made
/// again, and may now be won by the other stat (the rulings). A creature that is no longer on the battlefield gets no counter.
fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(entered) = item.subject else { return Ok(()) };
    if is_greater(g, item.object, entered) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// Evolve, with the reminder text its cards print.
pub const EVOLVE: AbilityDef = triggered(
    when_asked(EventMask::ZONE_CHANGE, greater_creature_you_control_enters),
    put_counter,
    "Evolve (Whenever a creature you control enters, if that creature has greater power or toughness than this creature, put a +1/+1 counter on this creature.)",
);
