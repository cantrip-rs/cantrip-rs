use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::plus_one_counter_on_this,
        "Celebration — Whenever this creature attacks, if two or more nonland permanents entered the battlefield under your control this turn, put a +1/+1 counter on this creature.",
    )
    .condition(history::you_celebrate)],
    ..oracle::ASH_PARTY_CRASHER
};
