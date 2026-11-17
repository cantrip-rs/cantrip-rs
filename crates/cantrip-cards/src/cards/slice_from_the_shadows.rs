use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -X/-X until end of turn" (CR 611.2a, 107.3a): X is the value chosen as the spell was cast, and a target
/// no longer legal as it resolves means the spell does not resolve (CR 608.2b). "This spell can't be countered" is the core
/// keyword of its content file (CR 101.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = item.x as i32;
    pump(g, item.object, creature, -x, -x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::SLICE_FROM_THE_SHADOWS
};
