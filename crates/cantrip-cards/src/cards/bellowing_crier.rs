use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::loot, "When this creature enters, draw a card, then discard a card.")],
    ..oracle::BELLOWING_CRIER
};
