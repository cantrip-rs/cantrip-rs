use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{have_trample_and_lifelink, plus_pt};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchant creature"; "Enchanted creature gets +2/+2 and has trample and lifelink" (CR 303.4, 613.4c, 613.1f, 702.19, 702.15).
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Enchanted creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, have_trample_and_lifelink, "Enchanted creature has trample and lifelink."),
    ],
    ..oracle::UNFLINCHING_COURAGE
};
