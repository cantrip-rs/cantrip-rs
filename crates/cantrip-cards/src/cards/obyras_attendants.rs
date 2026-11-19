use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715), Desperate Parry: "Target creature gets -4/-0 until end of turn." (CR 611.2a, 613.4c). A target that is no
/// longer legal (CR 608.2b) is left alone.
pub static DESPERATE_PARRY: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets::<-4, 0>), ..oracle::DESPERATE_PARRY };

pub static CARD: CardDef = CardDef { adventure: Some(&DESPERATE_PARRY), ..oracle::OBYRAS_ATTENDANTS };

pub static EXTRA: &[&CardDef] = &[&DESPERATE_PARRY];
