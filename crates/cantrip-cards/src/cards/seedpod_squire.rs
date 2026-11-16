use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::target_creature_gets::<1, 1>,
        "Whenever this creature attacks, target creature you control without flying gets +1/+1 until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL_WITHOUT_FLYING])],
    ..oracle::SEEDPOD_SQUIRE
};
