use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::each_opponent_loses_life_you_gain_life::<2>,
        "When this creature enters, each opponent loses 2 life and you gain 2 life.",
    )],
    ..oracle::SNEERING_SHADEWRITER
};
