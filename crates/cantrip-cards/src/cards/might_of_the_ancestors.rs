use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::BEGINNING_OF_YOUR_COMBAT};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::target_creature_gets_and_gains_vigilance::<2, 0>,
        "At the beginning of combat on your turn, target creature you control gets +2/+0 and gains vigilance until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::MIGHT_OF_THE_ANCESTORS
};
