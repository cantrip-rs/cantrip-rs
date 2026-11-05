use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "This spell costs {1} more to cast for each target beyond the first" (CR 601.2f): read once the targets are announced
/// (CR 601.2c), so zero targets or one costs nothing more. Its mana value is its mana cost's alone (CR 202.3).
fn more_per_extra_target(_: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let chosen = targets.iter().filter(|t| **t != Target::None).count();
    if chosen > 1 { CostDelta::more((chosen - 1) as u32) } else { CostDelta::NONE }
}

/// "Fireball deals X damage divided evenly, rounded down, among any number of targets": divided as it resolves, among the
/// targets still legal then (CR 608.2b; its rulings), each dealt X divided by how many those are, rounded down, all at once
/// (CR 120.3). With no legal target it deals none.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let legal: Vec<Target> = (0..item.target_count as usize).filter(|&slot| g.target_legal(item, slot)).map(|slot| g.state().target(item, slot)).collect();
    if legal.is_empty() {
        return Ok(());
    }
    // Divided in u32: the targets are a count of objects, which may pass a u16 (RX-1); each share is at most X.
    let each = (u32::from(item.x) / count_u32(legal.len())) as u16;
    if each > 0 {
        let hits: Vec<(ObjectId, Target, i64)> = legal.into_iter().map(|t| (item.object, t, i64::from(each))).collect();
        g.damage_together(&hits);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY.between(0, TargetSpec::ANY_NUMBER)], effect),
    own_cost_changes: &[more_per_extra_target],
    ..oracle::FIREBALL
};
