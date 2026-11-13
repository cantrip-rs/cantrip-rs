use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(SACRIFICE_THIS, effects::destroy_first_target, "Sacrifice this creature: Destroy target enchantment.")
        .targets(&[targets::ENCHANTMENT])],
    ..oracle::FELIDAR_CUB
};
