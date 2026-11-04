use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{1}{W}, {T}: Tap target nonenchantment creature." (CR 602.2, 701.26a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{W}").and(TAP), effects::tap_targets, "{1}{W}, {T}: Tap target nonenchantment creature.")
        .targets(&[targets::NONENCHANTMENT_CREATURE])],
    ..oracle::COEURL
};
