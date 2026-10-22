use cantrip_api::*;
use cantrip_mechanics::{effects::each_opponent_discards_a_card, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, each_opponent_discards_a_card, "When this creature enters, each opponent discards a card.")],
    ..oracle::RANK_RAT
};
