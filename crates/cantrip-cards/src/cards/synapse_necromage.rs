use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        tokens::create_fungi_that_cant_block::<2>,
        "When this creature dies, create two 1/1 black Fungus creature tokens with \"This token can't block.\"",
    )],
    ..oracle::SYNAPSE_NECROMAGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::FUNGUS_CANT_BLOCK];
