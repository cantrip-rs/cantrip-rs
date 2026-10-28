use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +0/+3 and gains hexproof until end of turn." Both are continuous effects from the
/// resolving spell (CR 611.2a); a spell whose target has become illegal does nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 0, 3);
        effects::gain_hexproof_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], effect), ..oracle::CHASE_INSPIRATION };
