use cantrip_api::*;
use cantrip_mechanics::{attach, effects, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{R}"),
        effects::attached_creature_gets::<1, 0>,
        "{R}: Enchanted creature gets +1/+0 until end of turn.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        attach::attached_object,
        statics::plus_pt_saturating::<0, 2>,
        "Enchanted creature gets +0/+2.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::STONEHANDS
};
