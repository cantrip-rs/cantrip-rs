use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip_cost};
use cantrip_mechanics::manifest_dread::manifest_dread_then_attach;
use cantrip_mechanics::statics::{have_deathtouch_and_lifelink, plus_pt};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, manifest_dread_then_attach, "When this Equipment enters, manifest dread, then attach this Equipment to that creature."),
        equip_cost(sacrifice(1, targets::filters::creature), "Equip\u{2014}Sacrifice a creature"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Equipped creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, have_deathtouch_and_lifelink, "Equipped creature has deathtouch and lifelink."),
    ],
    ..oracle::DISSECTION_TOOLS
};
