use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip, equip_quality};
use cantrip_mechanics::{lands, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        // "Equip Detective" (CR 702.6a): a Detective creature you control, "Detective" read through this card's words (CR 612.2).
        equip_quality(mana("{1}"), &[target(targets::creature_of_subtype_you_control::<{ lands::sub(Subtype::Detective) }>)], "Equip Detective {1}"),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, statics::plus_pt::<1, 2>, "Equipped creature gets +1/+2.")],
    ..oracle::THINKING_CAP
};
