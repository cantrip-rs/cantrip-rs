use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{discard_chosen, effects, threshold};
use cantrip_oracle as oracle;

/// "Threshold — Whenever this creature deals combat damage to a player, draw a card. Then discard a card unless there are
/// seven or more cards in your graveyard" (CR 207.2c, 510.2, 121.1, 701.9): threshold is not a condition on the trigger but is
/// asked as the effect carries on after the draw (a draw never changes the graveyard), and the discard is chosen from the hand
/// as it then is only when the graveyard is below seven.
fn loot_unless_threshold(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    if !threshold(g, item.controller) {
        discard_chosen(g, item.controller, item.object, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        loot_unless_threshold,
        "Threshold \u{2014} Whenever this creature deals combat damage to a player, draw a card. Then discard a card unless there are seven or more cards in your graveyard.",
    )],
    ..oracle::SHORELINE_LOOTER
};
