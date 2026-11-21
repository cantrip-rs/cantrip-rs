use cantrip_api::*;
use cantrip_mechanics::{history, prepare, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        prepare::become_prepared,
        "At the beginning of your end step, if two or more creatures died this turn, this creature becomes prepared.",
    )
    .condition(history::n_or_more_creatures_died_this_turn::<2>)],
    enters: Some(prepare::enters_prepared),
    prepare: Some(&crate::demonic_tutor::CARD),
    ..oracle::EMERITUS_OF_WOE
};
