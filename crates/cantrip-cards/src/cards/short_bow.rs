use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_reach, have_vigilance, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{1}"), "Equip {1}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_reach, "Equipped creature has reach."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance."),
    ],
    ..oracle::SHORT_BOW
};
