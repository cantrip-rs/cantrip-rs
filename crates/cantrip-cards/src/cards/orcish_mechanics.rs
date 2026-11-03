use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(sacrifice(1, targets::filters::artifact)),
        effects::damage_first_target::<2>,
        "{T}, Sacrifice an artifact: This creature deals 2 damage to any target.",
    )
    .targets(&[targets::ANY])],
    ..oracle::ORCISH_MECHANICS
};
