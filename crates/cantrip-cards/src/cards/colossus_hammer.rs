use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{lose_flying, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{8}"), "Equip {8}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<10, 10>, "Equipped creature gets +10/+10."),
        static_ability(Layer::Ability, attached_object, lose_flying, "Equipped creature loses flying."),
    ],
    ..oracle::COLOSSUS_HAMMER
};
