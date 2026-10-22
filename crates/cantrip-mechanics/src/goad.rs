//! Goad (CR 701.15a): "Goad target creature. (Until your next turn, that creature attacks each combat if able and attacks a
//! player other than you if able.)" The duration the card prints is the card's own (CR 611.2a); the goading player is the
//! controller of the effect's source. Dack Fayden, Helping Hand goads for the rest of the game; the Undercity dungeon's
//! Arena goads until its controller's next turn.

use cantrip_api::*;

/// Goad `target` (CR 701.15a): the creature must attack each combat if able and attacks a player other than the goading
/// player if able, for as long as `duration` lasts.
pub fn goad(g: &mut dyn Game, source: ObjectId, target: ObjectId, duration: Duration) {
    g.add_effect(source, target, EffectKind::Goaded, duration);
}
