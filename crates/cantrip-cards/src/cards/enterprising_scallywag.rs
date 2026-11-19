use cantrip_api::*;
use cantrip_mechanics::{history, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        tokens::create_a_treasure,
        "At the beginning of your end step, if you descended this turn, create a Treasure token.",
    )
    .condition(history::descended_this_turn)],
    ..oracle::ENTERPRISING_SCALLYWAG
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
