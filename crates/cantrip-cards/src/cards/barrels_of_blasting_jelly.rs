use cantrip_api::*;
use cantrip_mechanics::{effects, mana, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana::ONE_FOR_ANY_COLOR_ONCE_EACH_TURN,
        ability(cost("{5}, {T}").and(SACRIFICE_THIS), effects::damage_first_target::<5>, "{5}, {T}, Sacrifice this artifact: It deals 5 damage to target creature.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::BARRELS_OF_BLASTING_JELLY
};
