use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on Donatello" (CR 122.1): on him if he is still on the battlefield as the ability resolves.
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::ARTIFACT_YOU_CONTROL_ENTERS, counter, "Whenever an artifact you control enters, put a +1/+1 counter on Donatello.")],
    ..oracle::DONATELLO_WAY_WITH_MACHINES
};
