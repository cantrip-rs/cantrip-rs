use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        |g, item| effects::scry(g, item, 1),
        "Disappear \u{2014} At the beginning of your end step, if a permanent left the battlefield under your control this turn, scry 1.",
    )
    .condition(history::disappear)],
    ..oracle::INSECTOID_EXTERMINATOR
};
