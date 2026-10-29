use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<3>),
    abilities: &[
        ability(cost("{1}{U}").and(REMOVE_A_COUNTER), |g, item| effects::draw(g, item, 1), "{1}{U}, Remove a counter from this creature: Draw a card."),
        ability(
            cost("{2}{U}").and(remove_counters_among(2, targets::filters::this_permanent)),
            effects::tap_and_stun::<1>,
            "{2}{U}, Remove two counters from this creature: Tap target creature. Put a stun counter on it.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::LOCH_MARE
};
