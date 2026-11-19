use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        effects::plus_one_counter_on_this,
        "At the beginning of your end step, if you descended this turn, put a +1/+1 counter on this creature.",
    )
    .condition(history::descended_this_turn)],
    ..oracle::CHILD_OF_THE_VOLCANO
};
