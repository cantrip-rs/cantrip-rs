use cantrip_api::*;
use cantrip_mechanics::effects::first_target_gains_vigilance_and_gets_plus_x_creatures_you_control;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        first_target_gains_vigilance_and_gets_plus_x_creatures_you_control,
        "When this creature enters, until end of turn, target creature you control gains vigilance and gets +X/+X, where X is the number of creatures you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::TREEGUARD_DUO
};
