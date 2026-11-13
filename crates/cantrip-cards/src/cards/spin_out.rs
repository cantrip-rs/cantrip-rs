use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_OR_VEHICLE], effects::destroy_first_target), ..oracle::SPIN_OUT };
