use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you put one or more +1/+1 counters on a creature you control, put a supply counter on this enchantment"
/// (`you_put_plus_one_counters_on_a_creature_you_control`, CR 122.6a, 603.2c): one trigger per creature the counters went on in
/// a batch (`batch_by_data` on the creature), however many were put on it; only +1/+1 counters count.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::COUNTERS, triggers::you_put_plus_one_counters_on_a_creature_you_control),
            effects::counters_on_self::<{ CounterKind::Supply as usize }, 1>,
            "Whenever you put one or more +1/+1 counters on a creature you control, put a supply counter on this enchantment.",
        )
        .once_per_batch()
        .batch_by_data(),
        ability(cost("{2}").and(remove_counters(CounterKind::Supply, 1)), |g, item| effects::draw(g, item, 1), "{2}, Remove a supply counter from this enchantment: Draw a card."),
    ],
    ..oracle::STOCKING_THE_PANTRY
};
