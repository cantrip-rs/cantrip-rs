use cantrip_api::*;
use cantrip_mechanics::effects::target_creature_gets_plus_x_x_for_this_power;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_BECOMES_TAPPED,
        target_creature_gets_plus_x_x_for_this_power,
        "Whenever this creature becomes tapped, another target creature you control gets +X/+X until end of turn, where X is this creature's power.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::HEMOSYMBIC_MITE
};
