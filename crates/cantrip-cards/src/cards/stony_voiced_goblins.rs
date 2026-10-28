use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, effects::each_opponent_discards_a_card, "When this creature enters, each opponent discards a card.")],
    ..oracle::STONY_VOICED_GOBLINS
};
