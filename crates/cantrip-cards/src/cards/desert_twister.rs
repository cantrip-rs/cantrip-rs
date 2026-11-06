use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target permanent." (CR 608.2).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PERMANENT], effects::destroy_first_target),
    ..oracle::DESERT_TWISTER
};
