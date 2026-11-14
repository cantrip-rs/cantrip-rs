use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::gain_life_then_draw::<2, 2>,
        "When this creature enters, you gain 2 life and draw two cards.",
    )],
    ..oracle::CLOUDBLAZER
};
