use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;
use cantrip_mechanics::targets::filters;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(sacrifice(1, filters::creature)),
            effects::gain_life_equal_sacrificed_creatures_toughness,
            "{T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.",
        ),
    ],
    ..oracle::DIAMOND_VALLEY
};
