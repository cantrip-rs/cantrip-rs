use cantrip_api::*;
use cantrip_mechanics::tokens::{self, MUTAGEN};
use cantrip_mechanics::triggers::THIS_LEAVES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_LEAVES, tokens::create_mutagen, "When this creature leaves the battlefield, create a Mutagen token.")],
    ..oracle::ZOO_ESCAPEES
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
