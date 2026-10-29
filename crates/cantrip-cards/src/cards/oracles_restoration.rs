use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+1 until end of turn. You draw a card and gain 1 life." With its one target illegal
/// the spell does not resolve (CR 608.2b), so you neither draw nor gain life.
fn restore(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 1, 1);
    g.end_sentence();
    g.draw(item.controller, 1);
    g.gain_life(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], restore), ..oracle::ORACLES_RESTORATION };
