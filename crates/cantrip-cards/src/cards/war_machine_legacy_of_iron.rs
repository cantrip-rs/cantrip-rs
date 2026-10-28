use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Another target creature you control gets +X/+0 until end of turn, where X is War Machine's power": X is found as the
/// ability resolves (last known information if he has left), and a negative power counts as 0 (CR 107.1b).
fn pump_by_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = g.chars(item.object).power.unwrap_or_default().max(0);
    cantrip_mechanics::pump(g, item.object, creature, x, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        pump_by_power,
        "At the beginning of combat on your turn, another target creature you control gets +X/+0 until end of turn, where X is War Machine's power.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::WAR_MACHINE_LEGACY_OF_IRON
};
