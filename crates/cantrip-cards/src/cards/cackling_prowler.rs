use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, YOUR_END_STEP};
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "Ward {2}" (CR 702.21a) is the shared mechanic. "Morbid — At the beginning of your end step, if a creature died this turn,
/// put a +1/+1 counter on this creature." is an intervening "if" (CR 603.4): the creature must have died this turn when the step
/// begins and again on resolution.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            YOUR_END_STEP,
            effects::plus_one_counter_on_this,
            "Morbid — At the beginning of your end step, if a creature died this turn, put a +1/+1 counter on this creature.",
        )
        .condition(history::a_creature_died_this_turn),
    ],
    ..oracle::CACKLING_PROWLER
};
