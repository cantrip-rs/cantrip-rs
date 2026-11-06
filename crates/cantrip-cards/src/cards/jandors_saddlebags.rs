use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}").and(TAP), effects::untap_targets,
        "{3}, {T}: Untap target creature.")
        .targets(&[targets::CREATURE])],
    ..oracle::JANDORS_SADDLEBAGS
};
