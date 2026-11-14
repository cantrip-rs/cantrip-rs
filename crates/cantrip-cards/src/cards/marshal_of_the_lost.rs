use cantrip_api::*;
use cantrip_mechanics::{attacking_creatures, pump, targets, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

/// "Target creature gets +X/+X until end of turn, where X is the number of attacking creatures" (CR 611.2a, 608.2c): X is counted as
/// the ability resolves, attackers of every player, the Marshal's own included; nothing if the target is no longer legal (CR 608.2b).
fn pump_by_attackers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = attacking_creatures(g) as i32;
    pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        pump_by_attackers,
        "Whenever you attack, target creature gets +X/+X until end of turn, where X is the number of attacking creatures.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::MARSHAL_OF_THE_LOST
};
