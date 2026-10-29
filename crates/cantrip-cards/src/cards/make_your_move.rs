use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_ENCHANTMENT_OR_CREATURE_WITH_POWER_4_OR_GREATER], effects::destroy_first_target),
    ..oracle::MAKE_YOUR_MOVE
};
