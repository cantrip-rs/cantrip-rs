use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{R}{R}"),
                effects::damage_target_and_you::<1, 1>,
                "{1}{R}{R}: This creature deals 1 damage to any target and 1 damage to you.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::BROTHERS_OF_FIRE
};
