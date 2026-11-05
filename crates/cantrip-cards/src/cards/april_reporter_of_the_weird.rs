use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT};
use cantrip_oracle as oracle;

/// "Draw that many cards, then discard a card" (CR 121.1, 701.9): as many cards as the combat damage April dealt to the player (the
/// trigger's `event_amount`), all drawn before the discard is chosen.
fn draw_that_many_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, u32::try_from(item.matched_damage_amount()).expect("draw quantity exceeds existing u32 instruction domain"));
    discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        draw_that_many_then_discard,
        "Whenever April deals combat damage to a player, draw that many cards, then discard a card.",
    ).damage_amount()],
    ..oracle::APRIL_REPORTER_OF_THE_WEIRD
};
