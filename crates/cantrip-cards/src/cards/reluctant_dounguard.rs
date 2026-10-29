use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::another_creature_you_control_enters;
use cantrip_oracle as oracle;

/// "Whenever another creature you control enters while this creature has a -1/-1 counter on it" (CR 603.4): asked as the
/// creature enters, with the counters this one has then; the effect checks again as it resolves.
fn another_creature_enters_while_it_has_a_counter(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.obj(me.source).counters(CounterKind::MinusOne) <= 0 {
        return None;
    }
    another_creature_you_control_enters(q, me, event)
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_creature_enters_while_it_has_a_counter),
        effects::remove_a_minus_one_counter_from_this,
        "Whenever another creature you control enters while this creature has a -1/-1 counter on it, remove a -1/-1 counter from this creature.",
    )],
    ..oracle::RELUCTANT_DOUNGUARD
};
