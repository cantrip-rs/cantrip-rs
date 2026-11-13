use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{B}").and(sacrifice(1, targets::filters::creature)),
        effects::gain_life_then_draw::<1, 1>,
        "{1}{B}, Sacrifice a creature: You gain 1 life and draw a card.",
    )],
    ..oracle::VAMPIRIC_RITES
};
