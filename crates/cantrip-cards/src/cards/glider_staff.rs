use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_flying, plus_pt};
use cantrip_mechanics::{airbend_targets, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, airbend_targets, "When this Equipment enters, airbend up to one target creature.")
            .targets(&[targets::CREATURE.optional()]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_flying, "Equipped creature has flying."),
    ],
    ..oracle::GLIDER_STAFF
};
