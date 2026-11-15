use cantrip_api::*;
use cantrip_mechanics::{pump, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control gets +X/+0 until end of turn, where X is this creature's power": X is found as the ability
/// resolves (CR 608.2h), the power the Captain has then, or last had on the battlefield if it has left (CR 113.7a); a power of
/// 0 or less adds nothing (CR 107.1b). It may target itself. A target no longer legal gets nothing (CR 608.2b).
fn pump_by_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = targets::power_or_last_known(g, item.object).max(0);
    pump(g, item.object, creature, x, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        pump_by_power,
        "At the beginning of combat on your turn, target creature you control gets +X/+0 until end of turn, where X is this creature's power.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::BRAMBLEGUARD_CAPTAIN
};
