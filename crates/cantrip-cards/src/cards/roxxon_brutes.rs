use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers, typecycling};
use cantrip_oracle as oracle;

/// Menace is a core keyword read from its characteristics.
/// "Whenever you draw your second card each turn, put a +1/+1 counter on target creature." (CR 120.3, 603.2): the target is
/// chosen as the ability goes on the stack, and nothing happens if it is no longer legal as it resolves (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_DRAW_YOUR_SECOND_CARD,
            effects::plus_one_counter_on_target,
            "Whenever you draw your second card each turn, put a +1/+1 counter on target creature.",
        )
        .targets(&[targets::CREATURE]),
        typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2}"),
    ],
    ..oracle::ROXXON_BRUTES
};
