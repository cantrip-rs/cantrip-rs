//! Recruit (CR 701.70a): "Draw a card, then discard a card. If you discarded a nonland card this way, create a 1/1 white
//! Human Soldier creature token." The Queen of Dale prints it.
//!
//! The player draws (an empty library draws nothing, CR 121.4, and loses at the next state check, CR 704.5b), then
//! discards a card they choose (CR 701.9); with an empty hand there is nothing to discard and so no token. The token is made
//! only for a card that was actually discarded: one its owner can't be made to discard stays (CR 101.2), and a card
//! discarded is judged by what it is where it went, a land card or not - and one a replacement put into a hidden zone
//! unrevealed is undefined, no nonland card (CR 701.9c, `hand::discarded_nonland_card`).

use crate::{choose_from_hand, hand, tokens};
use cantrip_api::*;

/// `player` recruits, on behalf of `source` (the object whose ability does it).
pub fn recruit(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    g.draw(player, 1);
    let chosen = choose_from_hand(g, player, source, 1)?;
    let discarded = g.discard_together(&chosen);
    if discarded.iter().any(|&card| hand::discarded_nonland_card(&*g, card)) {
        tokens::create_by_rule(g, player, &tokens::HUMAN_SOLDIER, 1, false)?;
    }
    Ok(())
}

/// The resolution of "[you] recruit" as an ability's whole effect: the ability's controller recruits, on behalf of its
/// source. The Queen of Dale, Celebrate the Mountain-king, Esgaroth Garrison, Great Gilded Boat, Lake-town Lookout, Long Lake Nuisance and The Mountain-king's
/// Return print it so.
pub fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    recruit(g, item.controller, item.object)
}
