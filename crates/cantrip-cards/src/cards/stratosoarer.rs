use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, typecycling};
use cantrip_oracle as oracle;

// Flying is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::first_target_gains_flying, "When this creature enters, target creature gains flying until end of turn.").targets(&[targets::CREATURE]),
        typecycling::basic_landcycling(mana("{1}{U}"), "Basic landcycling {1}{U}"),
    ],
    ..oracle::STRATOSOARER
};
