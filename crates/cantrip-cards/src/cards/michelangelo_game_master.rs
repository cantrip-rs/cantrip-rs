use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        effects::plus_one_counter_on_this,
        "Disappear \u{2014} At the beginning of your end step, if a permanent left the battlefield under your control this turn, put a +1/+1 counter on Michelangelo.",
    )
    .condition(history::disappear)],
    ..oracle::MICHELANGELO_GAME_MASTER
};
