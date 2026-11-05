use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_oracle as oracle;

/// Put a hone counter on each Equipment you control: those `you` control as the ability resolves (CR 608.2h), whatever
/// they are attached to or not. Each counter gives the creature its Equipment is attached to +1/+0 (CR 122.1j, the layers).
fn hone_each_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let puts: Vec<(ObjectId, CounterKind, i64)> =
        cantrip_mechanics::attach::equipment_controlled(g, you).into_iter().map(|id| (id, CounterKind::Hone, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        hone_each_equipment,
        "Whenever Dwalin enters or attacks, put a hone counter on each Equipment you control. (Each hone counter on an Equipment grants +1/+0 to equipped creature.)",
    )],
    ..oracle::DWALIN_WEAPONMASTER
};
