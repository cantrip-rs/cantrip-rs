use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, you gain X life, where X is the number of cards in your hand minus 4." (CR 608.2, 119.3);
/// a negative X counts as 0 (CR 107.1b).
fn gain_x_life_where_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hand = cantrip_mechanics::cards_in_hand(g, item.controller).min(i32::MAX as usize) as i32;
    g.gain_life(item.controller, (hand - 4).max(0));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            gain_x_life_where_x,
            "At the beginning of your upkeep, you gain X life, where X is the number of cards in your hand minus 4.",
        ),
    ],
    ..oracle::IVORY_TOWER
};
