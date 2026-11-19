use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ENCHANTMENT_YOU_CONTROL_ENTERS,
        effects::tap_targets,
        "Whenever an enchantment you control enters, tap target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::RIMEFUR_REINDEER
};
