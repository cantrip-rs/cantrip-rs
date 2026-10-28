use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature", if it is still the permanent that triggered (CR 400.7).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_ARTIFACT_YOU_CONTROL_ENTERS,
        grow,
        "Whenever another artifact you control enters, put a +1/+1 counter on this creature.",
    )],
    ..oracle::MACHINESMITH_AUTOMATON
};
