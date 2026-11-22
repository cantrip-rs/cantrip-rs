use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "This creature can't attack alone" (CR 508.1c): a restriction on the whole declaration, not on one attack - it attacks
/// as one of two or more attackers, and no other creature's attack makes any difference.
pub static CARD: CardDef = CardDef {
    combat_groups: &[CombatGroupRule { role: CombatRole::Attacker, about: Some(filters::this_permanent), at_least: 2, at_most: u32::MAX }],
    ..oracle::PET_SEHLAT
};
