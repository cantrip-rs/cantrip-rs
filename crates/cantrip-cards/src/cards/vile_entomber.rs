use cantrip_api::*;
use cantrip_mechanics::{library, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        library::search_for_a_card_to_graveyard,
        "When this creature enters, search your library for a card, put that card into your graveyard, then shuffle.",
    )],
    ..oracle::VILE_ENTOMBER
};
