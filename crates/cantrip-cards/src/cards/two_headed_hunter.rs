use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Target creature gains double strike until end of turn."
pub static TWICE_THE_RAGE: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_gains_double_strike), ..oracle::TWICE_THE_RAGE };

pub static CARD: CardDef = CardDef { adventure: Some(&TWICE_THE_RAGE), ..oracle::TWO_HEADED_HUNTER };

pub static EXTRA: &[&CardDef] = &[&TWICE_THE_RAGE];
