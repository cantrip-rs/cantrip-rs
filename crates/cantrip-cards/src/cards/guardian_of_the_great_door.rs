use cantrip_api::*;
use cantrip_mechanics::{card_types, untapped_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // As an additional cost to cast this spell, tap four untapped artifacts, creatures, and/or lands you control.
    additional_cost: Some(Cost::tap_untapped(4, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature, CardType::Land]) }>)),
    ..oracle::GUARDIAN_OF_THE_GREAT_DOOR
};
