use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, effects::each_opponent_loses_1_life, "Whenever this creature attacks, each opponent loses 1 life.")],
    ..oracle::PULSE_TRACKER
};
