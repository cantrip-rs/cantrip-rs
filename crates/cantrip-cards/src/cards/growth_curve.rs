use cantrip_api::*;
use cantrip_mechanics::{double, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature you control, then double the number of +1/+1 counters on that creature"
/// (CR 122.1, 701.10e): nothing if the target is no longer legal (CR 608.2b). The count to double is read after the
/// counter is on it, so a creature with none ends with two.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    double::counters(g, &[creature], CounterKind::PlusOne);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], effect), ..oracle::GROWTH_CURVE };
