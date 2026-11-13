use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        library::search_for_a_card_to_hand,
        "When this creature enters, search your library for a card, put it into your hand, then shuffle.",
    )],
    ..oracle::RUNE_SCARRED_DEMON
};
