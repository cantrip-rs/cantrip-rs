use cantrip_api::*;
use cantrip_mechanics::{affinity, fight, lands, targets};
use cantrip_oracle as oracle;

/// "Affinity for Frogs" (CR 702.41a): this spell costs {1} less to cast for each permanent with the Frog subtype its controller
/// controls, the type read through the spell's own words (CR 612.2). "Target creature you control deals damage equal to twice its
/// power to target creature you don't control." (CR 120.3, 609.7, 608.2b): one-sided, both targets judged as it resolves, so with
/// either illegal then no damage is dealt (the 2024-07-26 ruling).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], fight::first_target_bites_second_for_twice_its_power),
    own_cost_changes: &[affinity::for_creature_type::<{ lands::sub(Subtype::Frog) }>],
    ..oracle::POLLIWALLOP
};
