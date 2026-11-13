use cantrip_api::*;
use cantrip_mechanics::{affinity, effects, lands};
use cantrip_oracle as oracle;

/// "Affinity for Cats" (CR 702.41a): costs {1} less for each Cat you control. "Creatures you control get +2/+2 until end of turn"
/// (CR 611.2c): the creatures it affects are those on the battlefield as it resolves, and no others.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::creatures_you_control_get::<2, 2>),
    own_cost_changes: &[affinity::for_creature_type::<{ lands::sub(Subtype::Cat) }>],
    ..oracle::CLAWS_OUT
};
