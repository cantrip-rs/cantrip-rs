use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_object, attached_to_it, tap_enchanted},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Enchant creature or Spacecraft" (CR 303.4a): a Spacecraft whether or not it is a creature right now. "You control
/// enchanted permanent" (CR 613.1b, layer 2): the Aura's controller controls what it is attached to, at the Aura's timestamp
/// (CR 613.7e). "Doesn't untap during its controller's untap step" is a rule of the Aura's (CR 502.3, 113.1a), by whoever
/// controls the permanent as it untaps - the Aura's controller, since they control it.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_OR_SPACECRAFT),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted permanent.")],
    statics: &[control_changing(attached_object, "You control enchanted permanent.")],
    untap_bans: &[attached_to_it],
    ..oracle::TRACTOR_BEAM
};
