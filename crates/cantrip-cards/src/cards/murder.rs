use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature." (CR 701.8)
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::destroy_first_target), ..oracle::MURDER };
