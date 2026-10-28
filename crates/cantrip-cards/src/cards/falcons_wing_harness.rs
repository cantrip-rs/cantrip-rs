use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, have_flying, plus_pt};
use cantrip_mechanics::triggers::{THIS_ENTERS, ward};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{2}{U}"), "Equip {2}{U}"),
    ],
    // The ward the equipped creature has is an ability this Equipment grants (CR 702.21a), so it is the creature's own and
    // goes when the Equipment does.
    delayed: &[ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_flying, "Equipped creature has flying."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has ward {1}."),
    ],
    ..oracle::FALCONS_WING_HARNESS
};
