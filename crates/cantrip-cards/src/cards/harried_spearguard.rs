use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

// Haste is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        tokens::create_rats_that_cant_block::<1>,
        "When this creature dies, create a 1/1 black Rat creature token with \"This token can't block.\"",
    )],
    ..oracle::HARRIED_SPEARGUARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
