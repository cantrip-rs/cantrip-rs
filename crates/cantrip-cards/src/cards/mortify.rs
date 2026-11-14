use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature or enchantment." (CR 115.1, 701.8). The spell is not modal: a target that is still a creature or an
/// enchantment when it resolves is destroyed whichever it was when targeted.
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::creature_or_enchantment)], effects::destroy_first_target),
    ..oracle::MORTIFY
};
