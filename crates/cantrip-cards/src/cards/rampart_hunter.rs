use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::target_creature_gets_and_gains_deathtouch::<2, 2>,
        "When this creature enters, target creature gets +2/+2 and gains deathtouch until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::RAMPART_HUNTER
};
