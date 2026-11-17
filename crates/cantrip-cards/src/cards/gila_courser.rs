use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_mechanics::{library, saddle};
use cantrip_oracle as oracle;

/// "Saddle 1" (CR 702.171a) is the shared mechanic; "attacks while saddled" is judged when the Courser attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            library::exile_top_card_play_until_end_of_next_turn,
            "Whenever this creature attacks while saddled, exile the top card of your library. Until the end of your next turn, you may play that card.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::GILA_COURSER
};
