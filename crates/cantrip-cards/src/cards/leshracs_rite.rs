use cantrip_api::*;
use cantrip_mechanics::{attach, lands, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            statics::have_landwalk::<{ lands::sub(Subtype::Swamp) }>,
            "Enchanted creature has swampwalk.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::LESHRACS_RITE
};
