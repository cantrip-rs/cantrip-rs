use cantrip_api::*;
use cantrip_mechanics::{card_types, effects, targets, untapped_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        tap_untapped(2, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature]) }>),
        effects::target_creature_gets::<2, 0>,
        "Tap two untapped artifacts and/or creatures you control: Target creature gets +2/+0 until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE])
    .sorcery_speed()],
    ..oracle::GOLDFURY_STRIDER
};
