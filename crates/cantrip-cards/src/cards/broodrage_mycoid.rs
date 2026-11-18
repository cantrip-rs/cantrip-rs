use cantrip_api::*;
use cantrip_mechanics::{history, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        tokens::create_fungi_that_cant_block::<1>,
        "At the beginning of your end step, if you descended this turn, create a 1/1 black Fungus creature token with \"This token can't block.\"",
    )
    .condition(history::descended_this_turn)],
    ..oracle::BROODRAGE_MYCOID
};

pub static EXTRA: &[&CardDef] = &[&tokens::FUNGUS_CANT_BLOCK];
