use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_OR_ANOTHER_CREATURE_YOU_CONTROL_WITH_FLYING_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_OR_ANOTHER_CREATURE_YOU_CONTROL_WITH_FLYING_ENTERS,
        effects::plus_one_counters_on_target::<1>,
        "Whenever this creature or another creature you control with flying enters, put a +1/+1 counter on target creature you control without flying.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL_WITHOUT_FLYING])],
    ..oracle::PLUMECREED_MENTOR
};
