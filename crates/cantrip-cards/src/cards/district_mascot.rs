use cantrip_api::*;
use cantrip_mechanics::effects::{destroy_first_target, enters_with_a_counter, plus_one_counter_on_this};
use cantrip_mechanics::{saddle, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_a_counter::<{ CounterKind::PlusOne as usize }>),
    abilities: &[
        ability(
            cost("{1}{G}").and(remove_counters(CounterKind::PlusOne, 2)),
            destroy_first_target,
            "{1}{G}, Remove two +1/+1 counters from this creature: Destroy target artifact.",
        )
        .targets(&[targets::ARTIFACT]),
        triggered(triggers::THIS_ATTACKS_WHILE_SADDLED, plus_one_counter_on_this, "Whenever this creature attacks while saddled, put a +1/+1 counter on it."),
        saddle::ability(1, "Saddle 1"),
    ],
    ..oracle::DISTRICT_MASCOT
};
