use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        effects::plus_one_counter_on_target,
        "Whenever this creature enters or attacks, put a +1/+1 counter on target creature you control with power 2 or less.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL_WITH_POWER_2_OR_LESS])],
    ..oracle::HAAZDA_VIGILANTE
};
