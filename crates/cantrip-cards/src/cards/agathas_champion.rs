use cantrip_api::*;
use cantrip_mechanics::{bargain, fight, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    abilities: &[triggered(
        THIS_ENTERS,
        fight::this_fights_first_target,
        "When this creature enters, if it was bargained, it fights up to one target creature you don't control.",
    )
    .targets(&[targets::CREATURE_YOU_DONT_CONTROL.optional()])
    .condition(bargain::was_bargained)],
    ..oracle::AGATHAS_CHAMPION
};
