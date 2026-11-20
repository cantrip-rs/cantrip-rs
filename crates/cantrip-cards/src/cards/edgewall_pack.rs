use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        tokens::create_rats_that_cant_block::<1>,
        "When this creature enters, create a 1/1 black Rat creature token with \"This token can't block.\"",
    )],
    ..oracle::EDGEWALL_PACK
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
