use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        loses_all_abilities(
            attach::attached_object,
            "Enchanted creature gets -5/-0 and loses all abilities.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                cantrip_mechanics::statics::plus_pt_saturating::<-5, 0>,
                "Enchanted creature gets -5/-0 and loses all abilities.",
            )
            .continues(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::FRESH_START
};
