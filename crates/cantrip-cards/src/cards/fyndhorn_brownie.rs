use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}{G}").and(TAP),
                effects::untap_targets,
                "{2}{G}, {T}: Untap target creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::FYNDHORN_BROWNIE
};
