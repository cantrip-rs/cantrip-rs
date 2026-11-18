use cantrip_api::*;
use cantrip_mechanics::{card_types, effects, untapped_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        Cost::tap_untapped(2, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature]) }>),
        effects::plus_one_counter_on_this,
        "Tap two untapped artifacts and/or creatures you control: Put a +1/+1 counter on this creature. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    ..oracle::ADAPTIVE_GEMGUARD
};
