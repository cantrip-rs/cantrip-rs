use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more other creatures you control die" (CR 700.4, 603.2c, 603.10a): the creatures are judged as they last were
/// on the battlefield; those that die together are one batch and trigger it once.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_YOU_CONTROL_DIES,
        effects::plus_one_counter_on_this,
        "Whenever one or more other creatures you control die, put a +1/+1 counter on this creature.",
    )
    .once_per_batch()],
    ..oracle::VENGEFUL_TOWNSFOLK
};
