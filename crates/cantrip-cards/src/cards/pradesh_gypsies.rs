use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{G}").and(TAP),
                effects::target_creature_gets::<-2, 0>,
                "{1}{G}, {T}: Target creature gets -2/-0 until end of turn.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::PRADESH_GYPSIES
};
