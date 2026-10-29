use cantrip_api::*;
use cantrip_mechanics::library::exile_top_card_play_until_end_of_next_turn;
use cantrip_mechanics::{triggers, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            exile_top_card_play_until_end_of_next_turn,
            "When this creature enters, exile the top card of your library. Until the end of your next turn, you may play that card.",
        ),
        typecycling::basic_landcycling(mana("{1}{R}"), "Basic landcycling {1}{R}"),
    ],
    ..oracle::KULRATH_ZEALOT
};
