//! Ingest (CR 702.115a), a triggered ability: "Whenever this creature deals combat damage to a player, that player exiles
//! the top card of their library." Each instance triggers separately (CR 702.115b). Culling Drone and Mist Intruder print it.
//!
//! A card with ingest lists `TRIGGER` among its `abilities`.

use crate::top_of_library;
use crate::triggers::deals_combat_damage_to_player_subject;
use cantrip_api::*;

/// The damaged player (the trigger's subject) exiles the top card of their library, face up; with none, nothing happens
/// and they lose nothing (the rulings).
fn exile_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(player) = item.subject
        && let Some(&card) = top_of_library(g, player, 1).first()
    {
        g.exile(card);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(
    when(EventMask::DAMAGE, deals_combat_damage_to_player_subject),
    exile_the_top_card,
    "Ingest (Whenever this creature deals combat damage to a player, that player exiles the top card of their library.)",
);
