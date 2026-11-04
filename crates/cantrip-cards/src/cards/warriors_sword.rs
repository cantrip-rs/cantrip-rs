use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{5}"), "Equip {5}")],
    statics: &[
        static_ability(Layer::Type, attached_object, cantrip_mechanics::statics::also_a::<{ cantrip_mechanics::lands::sub(Subtype::Warrior) }>, "Equipped creature is a Warrior in addition to its other types."),
        static_ability(Layer::PtModify, attached_object, plus_pt::<3, 2>, "Equipped creature gets +3/+2."),
    ],
    ..oracle::WARRIORS_SWORD
};

pub static EXTRA: &[&CardDef] = &[&HERO];
