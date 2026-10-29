use cantrip_api::*;
use cantrip_mechanics::library::exile_top_card_play_until_end_of_next_turn;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        exile_top_card_play_until_end_of_next_turn,
        "When this creature dies, exile the top card of your library. Until the end of your next turn, you may play that card.",
    )],
    ..oracle::SIZZLING_CHANGELING
};
