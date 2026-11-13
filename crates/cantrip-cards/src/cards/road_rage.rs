use cantrip_api::*;
use cantrip_mechanics::{damage_target, mounts_and_vehicles_you_control, targets};
use cantrip_oracle as oracle;

/// "Road Rage deals X damage to target creature or planeswalker, where X is 2 plus the number of Mounts and Vehicles you control."
/// (CR 120.3, 107.3, 608.2b): X is counted as the spell resolves, a permanent that is both a Mount and a Vehicle once; nothing happens
/// if the target is no longer legal.
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = 2 + mounts_and_vehicles_you_control(g, item.object, item.controller).len() as u32;
    damage_target(g, item, 0, x);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_OR_PLANESWALKER], damage), ..oracle::ROAD_RAGE };
