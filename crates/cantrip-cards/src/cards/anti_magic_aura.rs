use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    targeting_rules: &[
        attach::ENCHANTED_CANT_BE_TARGET_OF_SPELLS,
        attach::ENCHANTED_NO_OTHER_AURAS,
    ],
    ..oracle::ANTI_MAGIC_AURA
};
