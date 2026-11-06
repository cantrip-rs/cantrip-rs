//! "Untap up to N lands" (CR 701.26a).

use crate::choose;
use cantrip_api::*;

/// The controller untaps up to `count` lands of their choice, chosen as the effect resolves and not targeted: any lands on
/// the battlefield, tapped or not. Nothing is asked when there are none. Time Spiral prints six, Teferi, Hero of
/// Dominaria two.
pub fn up_to_lands(g: &mut dyn Game, source: ObjectId, controller: PlayerId, count: u8) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    let chosen = choose::cards(g, controller, source, Purpose::Untap, lands, u32::from(count), true)?;
    g.untap_together(&chosen);
    Ok(())
}
