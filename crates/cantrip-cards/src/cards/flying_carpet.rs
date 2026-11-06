use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}").and(TAP),
        effects::first_target_gains_flying,
        "{2}, {T}: Target creature gains flying until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::FLYING_CARPET
};
