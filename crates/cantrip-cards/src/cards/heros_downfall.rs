use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effects::destroy_first_target), ..oracle::HEROS_DOWNFALL };
