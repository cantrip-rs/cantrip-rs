use cantrip_api::*;
use cantrip_oracle as oracle;

/// The creature both restrictions are about: this one.
fn itself(_: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id == asker.source
}

/// "This creature can't attack unless at least two other creatures attack. This creature can't block unless at least two
/// other creatures block." (CR 508.1c, 509.1b): a declaration with this creature in it has at least three creatures, this
/// one and two others.
pub static CARD: CardDef = CardDef {
    combat_groups: &[
        CombatGroupRule { role: CombatRole::Attacker, about: Some(itself), at_least: 3, at_most: u32::MAX },
        CombatGroupRule { role: CombatRole::Blocker, about: Some(itself), at_least: 3, at_most: u32::MAX },
    ],
    ..oracle::ORCISH_CONSCRIPTS
};
