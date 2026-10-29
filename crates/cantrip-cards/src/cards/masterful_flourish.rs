use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 and gains indestructible until end of turn" (CR 611.2a, 702.12): both come from the
/// spell, to the creature it targeted, if it is still a legal target as this resolves (CR 608.2b).
fn flourish(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, creature, 1, 0);
        effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], flourish),
    ..oracle::MASTERFUL_FLOURISH
};
