use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        effects::damage_first_target::<1>,
        "{T}: This creature deals 1 damage to target creature with flying.",
    )
    .targets(&[targets::CREATURE_WITH_FLYING])],
    ..oracle::CENTAUR_ARCHER
};
