use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP),
        effects::damage_first_target::<1>,
        "{1}, {T}: This creature deals 1 damage to target opponent.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::DEADEYE_DUELIST
};
