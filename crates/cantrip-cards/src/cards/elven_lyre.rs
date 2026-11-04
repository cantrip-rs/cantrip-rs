use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(TAP).and(SACRIFICE_THIS),
            effects::target_creature_gets::<2, 2>,
            "{1}, {T}, Sacrifice this artifact: Target creature gets +2/+2 until end of turn.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::ELVEN_LYRE
};
