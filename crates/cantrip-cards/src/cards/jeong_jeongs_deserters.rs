use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, effects::plus_one_counter_on_target, "When this creature enters, put a +1/+1 counter on target creature.")
        .targets(&[targets::CREATURE])],
    ..oracle::JEONG_JEONGS_DESERTERS
};
