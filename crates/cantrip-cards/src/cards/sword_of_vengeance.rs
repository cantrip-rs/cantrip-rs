use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_first_strike, have_haste, have_trample, have_vigilance, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{3}"), "Equip {3}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Equipped creature gets +2/+0."),
        static_ability(Layer::Ability, attached_object, have_first_strike, "Equipped creature has first strike."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance."),
        static_ability(Layer::Ability, attached_object, have_trample, "Equipped creature has trample."),
        static_ability(Layer::Ability, attached_object, have_haste, "Equipped creature has haste."),
    ],
    ..oracle::SWORD_OF_VENGEANCE
};
