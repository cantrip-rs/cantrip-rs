use cantrip_api::*;
use cantrip_mechanics::{attach::{attached_object, attached_to_it, tap_enchanted}, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_OR_VEHICLE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted permanent.")],
    statics: &[loses_all_abilities(attached_object, "Enchanted permanent loses all abilities.")],
    // "... doesn't untap during its controller's untap step" (CR 502.3): a rule of the Aura's, not an ability of the
    // permanent (CR 113.1a), so the permanent losing all its abilities leaves it in force (CR 613.1f).
    untap_bans: &[attached_to_it],
    ..oracle::FLOOD_THE_ENGINE
};
