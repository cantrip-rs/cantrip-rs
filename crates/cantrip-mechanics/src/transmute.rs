//! Transmute [cost] (CR 702.53a): "[Cost], Discard this card: Search your library for a card with the same mana value as the
//! discarded card, reveal that card, and put it into your hand. Then shuffle your library. Activate only as a sorcery." It is
//! an activated ability that functions only while the card is in a hand (CR 702.53a-b), and the discarded card is read as it
//! last existed there (CR 608.2h). Drift of Phantasms prints it.
//!
//! A card with transmute lists `transmute(cost, text)` among its `abilities`; `text` is the ability's full text, which a
//! scenario's `Ability:` matches.

use crate::{library, select};
use cantrip_api::*;

/// The search: a card, which may be left unfound (CR 701.23b), whose mana value is the discarded card's (CR 202.3).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::cards(select::up_to(1)).with(select::mana_value_exactly(g.mana_value(item.object)));
    library::find_to_hand(g, item.controller, item.object, rule, true)?;
    Ok(())
}

/// Transmute `mana` (CR 702.53a), from the hand, at sorcery speed.
pub const fn transmute(mana: ManaCost, text: &'static str) -> AbilityDef {
    ability(Cost::mana(mana).and(DISCARD_THIS), search, text).from(ZoneKind::Hand).sorcery_speed()
}
