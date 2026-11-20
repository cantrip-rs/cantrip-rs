use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::ARTIFACT_YOU_CONTROL_ENTERS,
            effects::two_plus_one_counters_on_this,
            "Whenever one or more artifacts you control enter, put two +1/+1 counters on this creature. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1)
        .once_per_batch(),
        triggered(
            triggers::ENCHANTMENT_YOU_CONTROL_ENTERS,
            |g, item| effects::draw(g, item, 1),
            "Whenever one or more enchantments you control enter, draw a card. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1)
        .once_per_batch(),
    ],
    ..oracle::ELVISH_ARCHIVIST
};
