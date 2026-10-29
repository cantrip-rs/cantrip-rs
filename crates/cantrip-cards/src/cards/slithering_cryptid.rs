use cantrip_api::*;
use cantrip_mechanics::tokens::{self, MUTAGEN};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_mutagen, "When this creature enters, create a Mutagen token.")],
    ..oracle::SLITHERING_CRYPTID
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
