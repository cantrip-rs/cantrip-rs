use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, tap_enchanted};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<20, 20>, "Enchanted creature gets +20/+20.")],
    ..oracle::COLOSSIFICATION
};
