use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        effects::plus_one_counter_on_target,
        "At the beginning of your end step, if an opponent lost life this turn, put a +1/+1 counter on target Vampire you control.",
    )
    .condition(history::an_opponent_lost_life)
    .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Vampire) }>)])],
    ..oracle::STROMKIRK_BLOODTHIEF
};
