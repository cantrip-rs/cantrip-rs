use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::manifest_dread::manifest_dread_then_attach;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, manifest_dread_then_attach, "When this Equipment enters, manifest dread, then attach this Equipment to that creature."),
        equip(mana("{4}"), "Equip {4}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::CONDUCTIVE_MACHETE
};
