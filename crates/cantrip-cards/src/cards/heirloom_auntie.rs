use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Surveil 1, then remove a -1/-1 counter from this creature" (CR 701.25a, 608.2c): the surveil is done first, then the counter
/// comes off, if this creature is still there with one.
fn surveil_then_remove_a_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)?;
    effects::remove_a_minus_one_counter_from_this(g, item)
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_YOU_CONTROL_DIES,
        surveil_then_remove_a_counter,
        "Whenever another creature you control dies, surveil 1, then remove a -1/-1 counter from this creature.",
    )],
    ..oracle::HEIRLOOM_AUNTIE
};
