use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target permanent to its owner's hand." (CR 608.2).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PERMANENT], effects::return_first_target_to_hand),
    ..oracle::BOOMERANG
};
