use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Exile target attacking creature." (CR 701.13a, 506.4): any attacking creature, an opponent's or your own.
pub static CARD: CardDef = CardDef { spell: spell(&[targets::ATTACKING_CREATURE], effects::exile_first_target), ..oracle::NOT_ON_MY_WATCH };
