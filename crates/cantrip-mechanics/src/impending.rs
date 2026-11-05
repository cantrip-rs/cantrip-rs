//! Impending N—[cost] (CR 702.176a): "If you cast this spell for its impending cost, it enters with N time counters and
//! isn't a creature until the last is removed. At the beginning of your end step, remove a time counter from it."
//! Overlord of the Balemurk and Overlord of the Mistmoors print it.
//!
//! A card with impending lists `way(cost, text)` as its only alternative cost, `enters::<N>` as its `enters`,
//! `NOT_A_CREATURE` among its `statics` and `REMOVE_A_TIME_COUNTER` among its `abilities`.

use cantrip_api::*;

const TIME: usize = CounterKind::Time as usize;

/// The alternative cost of impending: cast from the hand for `cost` (CR 118.9). It is the card's first way.
pub const fn way(cost: ManaCost, text: &'static str) -> WayDef {
    cantrip_api::way(Cost::mana(cost), ZoneKind::Hand, text)
}

/// "If you cast this spell for its impending cost, it enters with `N` time counters" (CR 702.176a, 614.1c): judged on the
/// spell it is entering from, which keeps the way it was cast for.
pub fn enters<const N: i16>(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if g.obj(entering.object).cast_way == 1 {
        entering.counters[TIME] += i64::from(N);
    }
    Ok(())
}

fn has_time_counter(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().obj(source).counters(CounterKind::Time) > 0
}

/// "Isn't a creature until the last is removed" (CR 613.1d): it loses the creature type and the creature types with it
/// (CR 205.1b), and a noncreature permanent has no power or toughness (CR 208.3).
pub fn not_a_creature(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    if !chars.is(CardType::Creature) {
        return;
    }
    chars.types = TypeSet(chars.types.0 & !(1 << CardType::Creature as u8));
    let mut kept = SubtypeSet::NONE;
    for s in chars.subtypes.iter().filter(|s| !s.is_creature_type()) {
        kept = kept.with(s);
    }
    chars.subtypes = kept;
    chars.power = None;
    chars.toughness = None;
}

pub const NOT_A_CREATURE: StaticDef = static_ability(Layer::Type, has_time_counter, not_a_creature, "As long as this permanent has a time counter on it, it isn't a creature.");

fn your_end_step(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::End } if q.state().turn.active == q.controller(source) => Fired::PLAIN,
        _ => None,
    }
}

/// Remove a time counter from it, if it is still the permanent that has one.
fn remove_a_time_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = item.object;
    if g.is_on_battlefield(this) && g.obj(this).became.is_none() && g.obj(this).counters(CounterKind::Time) > 0 {
        g.remove_counters(this, CounterKind::Time, 1);
    }
    Ok(())
}

pub const REMOVE_A_TIME_COUNTER: AbilityDef = triggered(when(EventMask::STEP_BEGAN, your_end_step), remove_a_time_counter, "At the beginning of your end step, remove a time counter from it.");
