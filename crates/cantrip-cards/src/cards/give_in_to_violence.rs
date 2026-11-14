use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 and gains lifelink until end of turn." (CR 611.2a, 702.15)
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_lifelink::<2, 2>), ..oracle::GIVE_IN_TO_VIOLENCE };
