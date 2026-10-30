use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{ward, ANOTHER_CREATURE_YOU_CONTROL_ENTERS};
use cantrip_oracle as oracle;

/// "While this creature has a -1/-1 counter on it": the counters it has as the other creature enters, asked with the trigger event
/// only, so a creature that enters when it has none triggers nothing. This is a "while", not an intervening "if" (CR 603.4).
fn has_a_minus_one_counter(q: &dyn Query, asker: &Asker) -> bool {
    q.state().obj(asker.source).counters(CounterKind::MinusOne) > 0
}

/// Trample is a core keyword.
pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<5>),
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(
            ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            effects::remove_a_minus_one_counter_from_this,
            "Whenever another creature you control enters while this creature has a -1/-1 counter on it, remove a -1/-1 counter from this creature.",
        )
        .only_if(has_a_minus_one_counter),
    ],
    ..oracle::BRISTLEBANE_BATTLER
};
