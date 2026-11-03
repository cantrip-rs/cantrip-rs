use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        effects::target_creature_gets::<1, 1>,
        "{T}: Target creature gets +1/+1 until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::WYLULI_WOLF
};
