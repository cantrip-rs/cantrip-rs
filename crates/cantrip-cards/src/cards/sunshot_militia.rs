use cantrip_api::*;
use cantrip_mechanics::{card_types, effects, untapped_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        tap_untapped(2, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature]) }>),
        effects::damage_each_opponent_n::<1>,
        "Tap two untapped artifacts and/or creatures you control: This creature deals 1 damage to each opponent. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    ..oracle::SUNSHOT_MILITIA
};
