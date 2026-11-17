use cantrip_api::*;
use cantrip_mechanics::{fight, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+1 until end of turn. Put a +1/+1 counter on it instead if it's a Mount. Then it deals damage equal
/// to its power to target creature you don't control" (CR 608.2c, 611.2a, 122.1a, 120.3): whether the creature is a Mount (the word read through this spell's own text, CR 612) is judged as the
/// spell resolves, through the layers, and a Mount gets the permanent counter in place of the temporary pump, never both; its power is
/// read after that, so either way the bite is one larger. With the first target illegal as the spell resolves nothing is dealt (it is no
/// source); with only the second illegal the creature still grows.
fn throw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let mount = g.words(item.object).subtype(Subtype::Mount);
    if g.chars(creature).has_subtype(mount) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    } else {
        pump(g, item.object, creature, 1, 1);
    }
    g.end_sentence();
    fight::targets_bite(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], throw), ..oracle::THROW_FROM_THE_SADDLE };
