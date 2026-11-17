use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, shuffle_enchanted_into_library, tap_enchanted},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Enchant creature. When this Aura enters, tap enchanted creature. Enchanted creature doesn't untap during its controller's
/// untap step. {U}{U}: Shuffle enchanted creature into its owner's library." (CR 303.4, 701.26a, 502.3, 701.24): only the Aura's
/// controller can activate the ability (ruling 2024-02-02), whoever controls the creature.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature."),
        ability(cost("{U}{U}"), shuffle_enchanted_into_library, "{U}{U}: Shuffle enchanted creature into its owner's library."),
    ],
    untap_bans: &[attached_to_it],
    ..oracle::DRAMATIC_ACCUSATION
};
