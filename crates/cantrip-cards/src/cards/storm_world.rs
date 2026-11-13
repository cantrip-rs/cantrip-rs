use cantrip_api::*;
use cantrip_mechanics::{cards_in_hand, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is 4 minus the number of cards in their hand." (CR 608.2h, 107.1b).
fn damage_for_small_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    let count = cards_in_hand(g, player);
    let damage = if count < 4 { (4 - count) as u32 } else { 0 };
    g.damage_together(&[(item.object, Target::Player(player), damage)]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_PLAYERS_UPKEEP,
        damage_for_small_hand,
        "At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is 4 minus the number of cards in their hand.",
    )],
    ..oracle::STORM_WORLD
};
