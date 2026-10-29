use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

// Convoke is the core's, read from the card's JSON.
/// "One or two target creatures each get +2/+1 until end of turn."
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.between(1, 2)], effects::targets_get::<2, 1>), ..oracle::APPEAL_TO_EIRDU };
