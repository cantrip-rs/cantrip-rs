use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::each_opponent_loses_life_you_gain_life_for_each_subtype_you_control::<{ lands::sub(Subtype::Shrine) }>,
            "When Northern Air Temple enters, each opponent loses X life and you gain X life, where X is the number of Shrines you control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Shrine) }>),
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever another Shrine you control enters, each opponent loses 1 life and you gain 1 life.",
        ),
    ],
    ..oracle::NORTHERN_AIR_TEMPLE
};
