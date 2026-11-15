use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::have_trample, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<2, 2>, "Enchanted creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, have_trample, "Enchanted creature has trample."),
    ],
    ..oracle::FRANTIC_STRENGTH
};
