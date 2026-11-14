use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;
use cantrip_mechanics::{targets, your_upkeep};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                sacrifice(1, targets::filters::creature),
                effects::gain_life_equal_sacrificed_creatures_toughness,
                "Sacrifice a creature: You gain life equal to the sacrificed creature's toughness. Activate only during your upkeep.",
            )
            .condition(your_upkeep),
    ],
    ..oracle::LIFE_CHISEL
};
