use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_enchanted},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Enchant creature. When this Aura enters, tap enchanted creature. Enchanted creature doesn't untap during its controller's
/// untap step." (CR 303.4, 701.26a, 502.3): the creature can still be untapped by an effect, only the untap step skips it.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    untap_bans: &[attached_to_it],
    ..oracle::CHARMED_SLEEP
};
