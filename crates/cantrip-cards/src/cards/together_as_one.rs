use cantrip_api::*;
use cantrip_mechanics::{damage_target, spent, targets};
use cantrip_oracle as oracle;

/// Converge: target player draws X cards, it deals X damage to any target, and you gain X life, where X is the
/// number of colors of mana spent to cast it (a copy had none spent). Draw, then damage, then life (CR 608.2c).
fn converge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = spent::colors_of_mana_spent(g, item.object);
    if g.target_legal(item, 0)
        && let Some(player) = item.target(0).player()
    {
        g.draw(player, x);
    }
    damage_target(g, item, 1, x as u16);
    g.gain_life(item.controller, x as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER, targets::ANY], converge),
    ..oracle::TOGETHER_AS_ONE
};
