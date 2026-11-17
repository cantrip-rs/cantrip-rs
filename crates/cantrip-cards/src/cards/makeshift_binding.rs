use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_and_you_gain_2_life, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        exile_target_until_leaves_and_you_gain_2_life,
        "When this enchantment enters, exile target creature an opponent controls until this enchantment leaves the battlefield. You gain 2 life.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::MAKESHIFT_BINDING
};
