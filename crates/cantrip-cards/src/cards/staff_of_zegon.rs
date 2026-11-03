use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Target creature gets -2/-0 until end of turn (CR 608.2b, 611.2a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(TAP),
        effects::target_creature_gets::<-2, 0>,
        "{3}, {T}: Target creature gets -2/-0 until end of turn.",
    ).targets(&[targets::CREATURE])],
    ..oracle::STAFF_OF_ZEGON
};
