use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        library::exile_top_card_play_this_turn,
        "At the beginning of your upkeep, exile the top card of your library. You may play that card this turn.",
    )],
    ..oracle::COUNT_ON_LUCK
};
