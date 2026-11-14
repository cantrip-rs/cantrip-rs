use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        effects::target_connives,
        "At the beginning of your end step, if a creature died this turn, target creature you control connives.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])
    .condition(history::a_creature_died_this_turn)],
    ..oracle::SCORPION_SEETHING_STRIKER
};
