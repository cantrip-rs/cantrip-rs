use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::YOU_GAIN_LIFE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_LIFE, effects::each_opponent_loses_1_life, "Whenever you gain life, each opponent loses 1 life.")],
    ..oracle::MARAUDING_BLIGHT_PRIEST
};
