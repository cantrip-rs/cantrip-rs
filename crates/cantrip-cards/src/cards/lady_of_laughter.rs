use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        |g, item| effects::draw(g, item, 1),
        "Celebration — At the beginning of your end step, if two or more nonland permanents entered the battlefield under your control this turn, draw a card.",
    )
    .condition(history::you_celebrate)],
    ..oracle::LADY_OF_LAUGHTER
};
