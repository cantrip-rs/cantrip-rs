use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+2 until end of turn." (CR 611.2a, 514.2, 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets::<3, 2>),
    ..oracle::INFURIATE
};
