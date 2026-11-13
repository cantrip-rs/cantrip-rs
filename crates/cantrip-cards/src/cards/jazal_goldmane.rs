use cantrip_api::*;
use cantrip_mechanics::{attacking_creatures, effects};
use cantrip_oracle as oracle;

/// "Attacking creatures you control get +X/+X until end of turn, where X is the number of attacking creatures" (CR 611.2c,
/// 608.2h): X is counted once, over every attacking creature of every player, as the ability resolves; the ones pumped are the
/// attacking creatures its controller controls at that moment, and none that attacks later. First strike is the core keyword
/// of its JSON.
fn attackers_get_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = attacking_creatures(g) as i32;
    let you = item.controller;
    effects::pump_creatures_where(g, item.object, |q, id| q.is_attacking(id) && q.controller(id) == you, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{W}{W}"),
        attackers_get_x,
        "{3}{W}{W}: Attacking creatures you control get +X/+X until end of turn, where X is the number of attacking creatures.",
    )],
    ..oracle::JAZAL_GOLDMANE
};
