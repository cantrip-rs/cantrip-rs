use cantrip_api::*;
use cantrip_mechanics::{banding, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            statics::colored_legendary_creatures_you_control::<{ Color::Red as u8 }>,
            statics::grant_delayed_ability,
            "Red legendary creatures you control have \"bands with other legendary creatures.\"",
        ),
    ],
    delayed: &[banding::WITH_OTHER_LEGENDARY_CREATURES],
    ..oracle::MOUNTAIN_STRONGHOLD
};
