use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Faerie) }>),
        effects::each_opponent_loses_1_life,
        "Whenever another Faerie you control enters, each opponent loses 1 life.",
    )],
    ..oracle::OBYRA_DREAMING_DUELIST
};
