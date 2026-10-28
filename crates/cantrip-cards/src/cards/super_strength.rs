use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{grant_delayed_ability, have_trample, plus_pt};
use cantrip_mechanics::{targets, triggers::ward};
use cantrip_oracle as oracle;

/// "Enchant creature"; "Enchanted creature gets +4/+4 and has trample and ward {1}" (CR 303.4, 613.4c, 702.19, 702.21): the ward
/// ability is granted to it (CR 613.1f), triggering for the enchanted creature's opponents.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    delayed: &[ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<4, 4>, "Enchanted creature gets +4/+4."),
        static_ability(Layer::Ability, attached_object, have_trample, "Enchanted creature has trample."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Enchanted creature has ward {1}."),
    ],
    ..oracle::SUPER_STRENGTH
};
