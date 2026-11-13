use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{3}, {T}").and(SACRIFICE_THIS),
            effects::damage_first_target::<3>,
            "{3}, {T}, Sacrifice this artifact: It deals 3 damage to target creature.",
        )
        .targets(&[targets::CREATURE]),
        ability(
            cost("{6}, {T}").and(SACRIFICE_THIS),
            effects::destroy_first_target,
            "{6}, {T}, Sacrifice this artifact: Destroy target creature or Vehicle.",
        )
        .targets(&[targets::CREATURE_OR_VEHICLE]),
    ],
    ..oracle::SCRAP_COMPACTOR
};
