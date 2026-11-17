use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Thunder Salvo deals X damage to target creature, where X is 2 plus the number of other spells you've cast this turn" (CR 120.3,
/// 601.2i, 608.2b): X is worked out once, as it resolves, from every spell its controller cast this turn before it, whether or not
/// those resolved, were countered or are still on the stack. A copy of Thunder Salvo is not cast (CR 707.10), so it counts the
/// original, and a spell cast while it was on the stack counts too.
fn deal_x_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others = g.history().casts_by(item.controller).filter(|cast| cast.spell != item.object).count() as u32;
    damage_target(g, item, 0, 2 + others);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], deal_x_damage),
    ..oracle::THUNDER_SALVO
};
