use cantrip_api::*;
use cantrip_mechanics::{combat, statics};
use cantrip_oracle as oracle;

/// "During your turn, creatures you control have hexproof" (CR 604.1, 611.3a, 702.11b): judged as the layers are applied. "Each creature
/// you control with toughness greater than its power assigns combat damage equal to its toughness rather than its power" (CR 510.1a),
/// judged as combat damage is assigned; the power itself is not changed (the ruling).
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::your_creatures_during_your_turn,
        statics::have_hexproof,
        "During your turn, creatures you control have hexproof.",
    )],
    damage_by_toughness: &[combat::your_creature_with_toughness_greater_than_power],
    ..oracle::BEDROCK_TORTOISE
};
