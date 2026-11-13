use cantrip_api::*;
use cantrip_mechanics::{effects, history::an_opponent_lost_life, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::each_opponent_discards_a_card,
        "When this creature enters, if an opponent lost life this turn, each opponent discards a card.",
    )
    .condition(an_opponent_lost_life)],
    ..oracle::BLOODTITHE_COLLECTOR
};
