use cantrip_api::*;
use cantrip_mechanics::{pump, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// Target creature gets +3/+0 and gains trample until end of turn (CR 611.2a).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 3, 0);
        trample_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], resolve),
    ..oracle::FANATICAL_FEVER
};
