use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[ability(
        cost("{1}{R}").and(REMOVE_A_COUNTER),
        effects::first_target_cant_block_this_turn,
        "{1}{R}, Remove a counter from this creature: Target creature can't block this turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE])
    .sorcery_speed()],
    ..oracle::BRAMBLEBACK_BRUTE
};
