use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{3}").and(TAP),
                effects::damage_first_target::<1>,
                "{3}, {T}: This artifact deals 1 damage to any target.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::ROD_OF_RUIN
};
