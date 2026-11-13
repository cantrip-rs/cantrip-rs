use cantrip_api::*;
use cantrip_mechanics::{blight, effects};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{R}").and(TAP).and(blight::cost(1)),
        effects::damage_each_opponent_n::<2>,
        "{1}{R}, {T}, Blight 1: This creature deals 2 damage to each opponent.",
    )],
    ..oracle::STING_SLINGER
};
