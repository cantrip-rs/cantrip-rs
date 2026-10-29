use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[ability(
        cost("{1}{W}").and(REMOVE_A_COUNTER),
        effects::target_creature_gains_indestructible,
        "{1}{W}, Remove a counter from this creature: Target creature gains indestructible until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE])
    .sorcery_speed()],
    ..oracle::BURDENED_STONEBACK
};
