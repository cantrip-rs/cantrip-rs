use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_enchanted},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Enchant creature" (CR 303.4a). "Enchanted creature doesn't untap during its controller's untap step." is a rule of the Aura's
/// (CR 502.3, 113.1a), by whoever controls the creature as it untaps.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    untap_bans: &[attached_to_it],
    ..oracle::STARLIGHT_SNARE
};
