use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_object, attached_to_it, tap_enchanted},
    effects, statics, targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Enchanted creature doesn't untap during its controller's untap step" is a rule of the Aura's (CR 502.3, 113.1a), not an ability of
/// the creature. The "{5}: Untap this creature" the creature has is an ability it is given (CR 613.1f), activated by whoever controls
/// the creature, its source the creature itself (CR 113.7).
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    delayed: &[ability(cost("{5}"), effects::untap_self, "{5}: Untap this creature.")],
    statics: &[static_ability(Layer::Ability, attached_object, statics::grant_delayed_ability, "Enchanted creature has \"{5}: Untap this creature.\"")],
    untap_bans: &[attached_to_it],
    ..oracle::RINGING_STRIKE_MASTERY
};
