use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::target_creature_gets::<2, 2>,
        "When this creature enters, another target creature or Vehicle you control gets +2/+2 until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_OR_VEHICLE_YOU_CONTROL])],
    ..oracle::FANG_GUARDIAN
};
