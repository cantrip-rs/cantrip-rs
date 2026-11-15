use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::survival(
        effects::gain_life_amount::<2>,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, you gain 2 life.",
    )],
    ..oracle::CAUTIOUS_SURVIVOR
};
