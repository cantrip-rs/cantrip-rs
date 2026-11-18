use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, enchanted_permanent_cant_activate, tap_enchanted},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::ARTIFACT_OR_CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted permanent.")],
    untap_bans: &[attached_to_it],
    activation_rules: &[enchanted_permanent_cant_activate],
    ..oracle::STUCK_IN_SUMMONERS_SANCTUM
};
