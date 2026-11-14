use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            statics::plus_pt::<0, 2>,
            "Enchanted creature gets +0/+2 and can't be the target of spells.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    targeting_rules: &[
        attach::ENCHANTED_CANT_BE_TARGET_OF_SPELLS,
    ],
    ..oracle::SPECTRAL_SHIELD
};
