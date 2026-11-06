use cantrip_api::*;
use cantrip_mechanics::{attach, statics::plus_pt_saturating, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        attach::attached_object,
        plus_pt_saturating::<-2, -1>,
        "Enchanted creature gets -2/-1.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::WEAKNESS
};
