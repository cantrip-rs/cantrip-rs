use cantrip_api::*;
use cantrip_mechanics::{combat, disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature is turned face up, return another target creature to its owner's hand." (CR 708.8, 603.6, 400.3): "another"
/// is any creature but this one, whoever controls it.
pub static CARD: CardDef = CardDef {
    block_rules: &[combat::power_three_or_greater_cant_block],
    ways: &[disguise::way(Cost::mana(mana("{1}{U}")), "Disguise {1}{U}")],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        effects::return_first_target_to_hand,
        "When this creature is turned face up, return another target creature to its owner's hand.",
    )
    .targets(&[targets::OTHER_CREATURE])],
    ..oracle::EXIT_SPECIALIST
};
