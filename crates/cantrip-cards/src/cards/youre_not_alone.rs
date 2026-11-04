use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 until end of turn. If you control three or more creatures, it gets +4/+4 until end of turn
/// instead." The creatures are counted as the spell resolves (CR 608.2h); "instead" makes it one effect (CR 608.2c), so the
/// creature gets +4/+4 alone, never +2/+2 as well, until end of turn (CR 611.2a).
fn not_alone(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let n = if creatures_you_control(g, item.controller) >= 3 { 4 } else { 2 };
    pump(g, item.object, creature, n, n);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], not_alone), ..oracle::YOURE_NOT_ALONE };
