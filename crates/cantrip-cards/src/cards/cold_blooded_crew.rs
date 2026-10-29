use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::two_plus_one_counters_on_target,
            "When this creature enters, put two +1/+1 counters on target creature.",
        )
        .targets(&[targets::CREATURE]),
        typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2}"),
    ],
    keywords: Keywords::of(&[Keyword::CantBeBlockedByPowerTwoOrLess]),
    ..oracle::COLD_BLOODED_CREW
};
