use cantrip_api::*;
use cantrip_mechanics::{flashback, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each of one or two target creatures": one counter on each target still legal as the spell
/// resolves (CR 608.2b, 115.1d).
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i64)> =
        targets::legal_objects(g, item).into_iter().map(|creature| (creature, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(1, 2)], counters),
    ways: &[flashback::way(cost("{3}{W}"), "Flashback {3}{W}")],
    ..oracle::PREDICTIVE_PREPARATIONS
};
