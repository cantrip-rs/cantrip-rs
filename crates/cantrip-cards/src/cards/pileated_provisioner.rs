use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::plus_one_counter_on_target,
        "When this creature enters, put a +1/+1 counter on target creature you control without flying.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL_WITHOUT_FLYING])],
    ..oracle::PILEATED_PROVISIONER
};
