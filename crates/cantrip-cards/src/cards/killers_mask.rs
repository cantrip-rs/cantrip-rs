use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::manifest_dread::manifest_dread_then_attach;
use cantrip_mechanics::statics::have_menace;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, manifest_dread_then_attach, "When this Equipment enters, manifest dread, then attach this Equipment to that creature."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::Ability, attached_object, have_menace, "Equipped creature has menace.")],
    ..oracle::KILLERS_MASK
};
