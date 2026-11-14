use cantrip_api::*;
use cantrip_mechanics::{target_player_sacrifices_a_creature, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[targets::PLAYER], target_player_sacrifices_a_creature), ..oracle::DIABOLIC_EDICT };
