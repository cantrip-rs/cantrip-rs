use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_DRAW_A_CARD, effects::plus_one_counter_on_this, "Whenever you draw a card, put a +1/+1 counter on Data.")],
    ..oracle::DATA_NEW_LIFE
};
