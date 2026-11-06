use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::set_first_target_base_pt::<0, 0>), ..oracle::MULTIPLY_BY_ZERO };
