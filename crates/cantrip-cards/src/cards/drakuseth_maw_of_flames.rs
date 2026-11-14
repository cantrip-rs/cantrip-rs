use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Drakuseth deals 4 damage to any target and 3 damage to each of up to two other targets." (CR 120.3, 608.2b): the first
/// target takes 4 and each other target still legal as it resolves takes 3, all dealt together by Drakuseth.
fn deal_four_and_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, u32)> = (0..item.target_count as usize)
        .filter(|&slot| g.target_legal(item, slot))
        .map(|slot| (item.object, g.state().target(item, slot), if slot == 0 { 4 } else { 3 }))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        deal_four_and_three,
        "Whenever Drakuseth, Maw of Flames attacks, it deals 4 damage to any target and 3 damage to each of up to two other targets.",
    )
    .targets(&[targets::ANY, targets::ANY_TARGET_OTHER_THAN_THOSE_CHOSEN.between(0, 2)])],
    ..oracle::DRAKUSETH_MAW_OF_FLAMES
};
