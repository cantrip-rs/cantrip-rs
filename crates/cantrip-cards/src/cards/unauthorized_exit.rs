use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target nonland permanent to its owner's hand. Surveil 1." (CR 701.25a)
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], effects::return_first_target_to_hand_then_surveil::<1>),
    ..oracle::UNAUTHORIZED_EXIT
};
