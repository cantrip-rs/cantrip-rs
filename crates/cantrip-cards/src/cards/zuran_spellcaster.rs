use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        effects::damage_first_target::<1>,
        "{T}: This creature deals 1 damage to any target.",
    )
    .targets(&[targets::ANY])],
    ..oracle::ZURAN_SPELLCASTER
};
