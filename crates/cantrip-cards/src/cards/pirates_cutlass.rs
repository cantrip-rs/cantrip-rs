use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target Pirate you control.")
            .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Pirate) }>)]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::PIRATES_CUTLASS
};
