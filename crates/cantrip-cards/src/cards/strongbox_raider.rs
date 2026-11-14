use cantrip_api::*;
use cantrip_mechanics::{library, raid};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[raid::enters(
        library::exile_top_two_choose_one_play_until_end_of_next_turn,
        "Raid \u{2014} When this creature enters, if you attacked this turn, exile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card.",
    )],
    ..oracle::STRONGBOX_RAIDER
};
