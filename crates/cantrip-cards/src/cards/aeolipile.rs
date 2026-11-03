use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}").and(SACRIFICE_THIS).and(TAP),
                effects::damage_first_target::<2>,
                "{1}, {T}, Sacrifice this artifact: It deals 2 damage to any target.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::AEOLIPILE
};
