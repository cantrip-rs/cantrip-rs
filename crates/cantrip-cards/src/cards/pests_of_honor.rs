use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers::BEGINNING_OF_YOUR_COMBAT};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::plus_one_counter_on_this,
        "Celebration — At the beginning of combat on your turn, if two or more nonland permanents entered the battlefield under your control this turn, put a +1/+1 counter on this creature.",
    )
    .condition(history::you_celebrate)],
    ..oracle::PESTS_OF_HONOR
};
