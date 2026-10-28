use cantrip_api::*;
use cantrip_mechanics::{library::exile_top_card_play_until_end_of_next_turn, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        exile_top_card_play_until_end_of_next_turn,
        "When this creature dies, exile the top card of your library. You may play it until the end of your next turn.",
    )],
    ..oracle::CLOCKWORK_PERCUSSIONIST
};
