use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

/// "Morbid — At the beginning of your end step, if a creature died this turn, put two +1/+1 counters on target creature you control."
/// (CR 603.4, 603.3d): an intervening "if" checked as the step begins and again as the ability resolves; the target is chosen as it
/// goes on the stack.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        effects::two_plus_one_counters_on_target,
        "Morbid — At the beginning of your end step, if a creature died this turn, put two +1/+1 counters on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])
    .condition(history::a_creature_died_this_turn)],
    ..oracle::NEEDLETOOTH_PACK
};
