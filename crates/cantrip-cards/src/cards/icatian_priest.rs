use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{W}{W}"),
        effects::target_creature_gets::<1, 1>,
        "{1}{W}{W}: Target creature gets +1/+1 until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::ICATIAN_PRIEST
};
