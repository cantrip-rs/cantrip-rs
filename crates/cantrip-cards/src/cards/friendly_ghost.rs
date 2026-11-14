use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

// Flying is a core keyword, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::target_creature_gets::<2, 4>, "When this creature enters, target creature gets +2/+4 until end of turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::FRIENDLY_GHOST
};
