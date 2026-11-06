use cantrip_api::*;
use cantrip_mechanics::library::reveal_top_to_hand;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("RevealAgain", "reveal another card?", "Put it into your hand and lose life equal to its mana value", "Reveal another", "Stop").no_default();

/// Reveal the top card of your library and put that card into your hand (a move, not a draw), lose life equal to its
/// mana value, and, while the library has a card left, you may repeat the process. Life is lost, not paid (CR 119.4
/// does not apply), so nothing stops a player at zero or below; an empty library ends the offer.
fn reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    while let Some((_, value)) = reveal_top_to_hand(g, player) {
        g.lose_life(player,(value as i32) as i32);
        if g.state().zone(Zone::library(player)).is_empty() {
            return Ok(());
        }
        if !cantrip_mechanics::effects::ask_may(g, player, item.object, Purpose::Ask(&ASK))? {
            return Ok(());
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], reveal), ..oracle::AD_NAUSEAM };
