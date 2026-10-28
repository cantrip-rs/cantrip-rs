use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap target creature. Put twice X stun counters on it" (CR 701.26a, 122.1d): X as it was cast. The counters go on whether
/// or not the creature was untapped. With its target gone the spell does not resolve (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, item.x as i16 * 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::PROCRASTINATE
};
