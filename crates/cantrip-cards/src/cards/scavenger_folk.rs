use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{G}").and(SACRIFICE_THIS).and(TAP),
                effects::destroy_first_target,
                "{G}, {T}, Sacrifice this creature: Destroy target artifact.",
            )
            .targets(&[targets::ARTIFACT]),
    ],
    ..oracle::SCAVENGER_FOLK
};
