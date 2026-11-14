use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +X/+X until end of turn. Then it fights up to one target creature you don't control." X is
/// the one the spell was cast with (CR 107.3, 601.2b).
fn might(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.x as i32;
    fight::pump_first_target_then_it_fights_second(g, item, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL.optional()], might),
    ..oracle::PRIMAL_MIGHT
};
