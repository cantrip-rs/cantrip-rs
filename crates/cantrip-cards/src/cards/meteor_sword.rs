use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::destroy_first_target, "When this Equipment enters, destroy target permanent.").targets(&[targets::PERMANENT]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<3, 3>, "Equipped creature gets +3/+3.")],
    ..oracle::METEOR_SWORD
};
