use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_vigilance, plus_pt};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this Equipment enters, return up to one other target nonland permanent to its owner's hand.")
            .targets(&[up_to_one(targets::other_nonland_permanent_filter)]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance."),
    ],
    ..oracle::BESPOKE_B
};
