use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            library::mill_two_then_counters_for_each_artifact_or_creature_card,
            "When this creature enters, mill two cards, then put a +1/+1 counter on this creature for each artifact and/or creature card in your graveyard.",
        ),
    ],
    ..oracle::CETI_EEL
};
