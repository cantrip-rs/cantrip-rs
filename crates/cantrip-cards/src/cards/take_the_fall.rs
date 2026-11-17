use cantrip_api::*;
use cantrip_mechanics::{effects, outlaw, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -1/-0 until end of turn. It gets -4/-0 until end of turn instead if you control an outlaw. Draw a card."
/// Whether you control an outlaw is checked as the spell resolves (CR 608.2h, 700.12a); "instead" makes it one effect (CR 608.2c),
/// so the creature gets -4/-0 alone, never -1/-0 as well (CR 611.2a). If the creature is no longer a legal target as the spell
/// resolves, the spell does not resolve (CR 608.2b) and nothing, the draw included, happens.
fn shrink_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let power = if outlaw::outlaws_controlled_by(g, item.controller) > 0 { -4 } else { -1 };
    pump(g, item.object, creature, power, 0);
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], shrink_then_draw), ..oracle::TAKE_THE_FALL };
