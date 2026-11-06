use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, targets::filters::land_type::<{ lands::sub(Subtype::Forest) }>),
        effects::gain_three_life,
        "Sacrifice a Forest: You gain 3 life.",
    )],
    ..oracle::DARK_HEART_OF_THE_WOOD
};
