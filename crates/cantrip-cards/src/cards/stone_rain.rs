use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target land." (CR 701.8a, 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::LAND], effects::destroy_first_target),
    ..oracle::STONE_RAIN
};
