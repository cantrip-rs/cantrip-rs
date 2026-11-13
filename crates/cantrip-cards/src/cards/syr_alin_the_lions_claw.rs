use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        effects::other_creatures_you_control_get::<1, 1>,
        "Whenever Syr Alin attacks, other creatures you control get +1/+1 until end of turn.",
    )],
    ..oracle::SYR_ALIN_THE_LIONS_CLAW
};
