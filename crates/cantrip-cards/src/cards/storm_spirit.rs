use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::damage_first_target::<2>, "{T}: This creature deals 2 damage to target creature.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::STORM_SPIRIT
};
