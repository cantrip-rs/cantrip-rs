use cantrip_api::*;
use cantrip_mechanics::tokens::{self, THOPTER};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        tokens::create_a_thopter,
        "When this creature enters, create a 1/1 colorless Thopter artifact creature token with flying.",
    )],
    ..oracle::NIMBLE_THOPTERIST
};

pub static EXTRA: &[&CardDef] = &[&THOPTER];
