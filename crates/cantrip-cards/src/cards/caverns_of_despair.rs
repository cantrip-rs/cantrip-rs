use cantrip_api::*;
use cantrip_oracle as oracle;

/// "No more than two creatures can attack each combat. No more than two creatures can block each combat." (CR 508.1c,
/// 509.1b).
pub static CARD: CardDef = CardDef {
    combat_groups: &[
        CombatGroupRule { role: CombatRole::Attacker, at_most: 2 },
        CombatGroupRule { role: CombatRole::Blocker, at_most: 2 },
    ],
    ..oracle::CAVERNS_OF_DESPAIR
};
