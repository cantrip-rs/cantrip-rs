use cantrip_api::*;
use cantrip_mechanics::triggers::creature_you_control_with_a_plus_one_counter_dies;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_plus_one_counters::<2>),
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_you_control_with_a_plus_one_counter_dies),
            effects::plus_one_counter_on_this,
            "Whenever a creature you control with a +1/+1 counter on it dies, put a +1/+1 counter on this artifact.",
        ),
        ability(
            cost("{T}"),
            effects::move_a_plus_one_counter_from_this_onto_target,
            "{T}: Move a +1/+1 counter from this artifact onto target creature. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE])
        .sorcery_speed(),
    ],
    ..oracle::EXPLORERS_CACHE
};
