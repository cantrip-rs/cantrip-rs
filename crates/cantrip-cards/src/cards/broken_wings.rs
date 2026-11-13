use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target artifact, enchantment, or creature with flying" (CR 701.8a), of either player.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_ENCHANTMENT_OR_CREATURE_WITH_FLYING], effects::destroy_first_target),
    ..oracle::BROKEN_WINGS
};
