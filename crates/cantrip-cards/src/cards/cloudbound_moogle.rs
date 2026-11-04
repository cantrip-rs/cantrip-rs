use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers, typecycling};
use cantrip_oracle as oracle;

/// "When this creature enters, put a +1/+1 counter on target creature." (CR 603.6a, 122.1); Plainscycling {2} (CR 702.29).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::plus_one_counter_on_target,
            "When this creature enters, put a +1/+1 counter on target creature.",
        )
        .targets(&[targets::CREATURE]),
        typecycling::ability(Subtype::Plains, mana("{2}"), "Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::CLOUDBOUND_MOOGLE
};
