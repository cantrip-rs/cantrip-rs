use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

// Convoke is the core's, read from the card's JSON.
/// "Destroy target attacking or blocking creature." (CR 701.8)
pub static CARD: CardDef = CardDef { spell: spell(&[targets::ATTACKING_OR_BLOCKING_CREATURE], effects::destroy_first_target), ..oracle::PROTECTIVE_RESPONSE };
