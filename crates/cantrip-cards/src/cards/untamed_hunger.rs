use cantrip_api::*;
use cantrip_mechanics::statics::{have_menace, plus_pt};
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "Enchant creature" (CR 303.4, 702.5): the Aura's spell targets a creature. "Enchanted creature gets +2/+1 and has menace"
/// (CR 613.1f, 613.4c): the bonus and the keyword in their layers.
pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<2, 1>, "Enchanted creature gets +2/+1 and has menace."),
        static_ability(Layer::Ability, attach::attached_object, have_menace, "Enchanted creature gets +2/+1 and has menace."),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::UNTAMED_HUNGER
};
