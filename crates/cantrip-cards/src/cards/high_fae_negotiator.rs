use cantrip_api::*;
use cantrip_mechanics::{bargain, effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, if it was bargained, each opponent loses 3 life and you gain 3 life." (CR 119.3): the opponents lose
/// the life together (CR 101.4), and the controller gains 3 whatever the loss turned out to be.
pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::each_opponent_loses_life_you_gain_life::<3>,
        "When this creature enters, if it was bargained, each opponent loses 3 life and you gain 3 life.",
    )
    .condition(bargain::was_bargained)],
    ..oracle::HIGH_FAE_NEGOTIATOR
};
