use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gains_deathtouch_and_indestructible), ..oracle::OFFER_IMMORTALITY };
