use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::plus_pt, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[static_ability(
        Layer::PtModify,
        attached_object,
        plus_pt::<1, 2>,
        "Enchanted creature gets +1/+2.",
    )],
    ..oracle::HOLY_STRENGTH
};
