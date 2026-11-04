use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP).and(sacrifice(1, targets::filters::creature)),
        effects::damage_first_target::<2>,
        "{1}, {T}, Sacrifice a creature: This artifact deals 2 damage to any target.",
    ).targets(&[targets::ANY])],
    ..oracle::SKULL_CATAPULT
};
