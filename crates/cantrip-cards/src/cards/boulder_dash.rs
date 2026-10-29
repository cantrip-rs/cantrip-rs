use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Boulder Dash deals 2 damage to any target and 1 damage to any other target." (CR 120.3, 608.2b): both are dealt together
/// by the spell, each target still legal as it resolves taking its own amount.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, u16)> =
        [2, 1].into_iter().enumerate().filter(|&(slot, _)| g.target_legal(item, slot)).map(|(slot, amount)| (item.object, g.state().target(item, slot), amount)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY, targets::ANY_TARGET_OTHER_THAN_THOSE_CHOSEN], effect),
    ..oracle::BOULDER_DASH
};
