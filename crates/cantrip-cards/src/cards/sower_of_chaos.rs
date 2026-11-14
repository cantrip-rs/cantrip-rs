use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{R}"), effects::first_target_cant_block_this_turn, "{2}{R}: Target creature can't block this turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::SOWER_OF_CHAOS
};
