use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        |g, item| effects::scry(g, item, 1),
        "At the beginning of your end step, if you descended this turn, scry 1.",
    )
    .condition(history::descended_this_turn)],
    ..oracle::RUIN_LURKER_BAT
};
