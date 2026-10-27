//! Surge [cost] (CR 702.117a): "You may pay [cost] rather than pay this spell's mana cost as you cast this spell if you or
//! one of your teammates has cast another spell this turn." Goblin Freerunner and Jwar Isle Avenger print it.
//!
//! A card with surge lists `way(cost, text)` among its `ways`: an alternative cost (CR 118.9) paid following CR 601.2b and
//! 601.2f-h, which names no zone, from the hand like evoke's. Cantrip's games have no teams - a duel, or a free-for-all of
//! more players, never a game between teams (CR 102.3, 810.1) - so no player has a teammate and "you or one of your
//! teammates" is you alone. "Another spell"
//! is any spell its caster has cast this turn - resolved, countered or still on the stack (the ruling) - asked of the
//! history as the cast is offered and begun, before this spell becomes cast (CR 601.2i), so this one is never among them.
//!
//! ```ignore
//! ways: &[surge::way(mana("{1}{R}"), "Surge {1}{R}")],
//! ```

use cantrip_api::*;

/// Whether the caster has cast another spell this turn (CR 702.117a, 601.2i): a copy is not cast (CR 707.10) and is not
/// in the history as a cast.
pub fn cast_another_spell_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().casts_by(asker.controller).next().is_some()
}

/// The alternative cost of surge: `cost` rather than the mana cost (CR 118.9), if its caster has cast another spell this
/// turn. `text` is the keyword as printed ("Surge {1}{R}").
pub const fn way(cost: ManaCost, text: &'static str) -> WayDef {
    cantrip_api::way(Cost::mana(cost), ZoneKind::Hand, text).condition(cast_another_spell_this_turn).kind(WayKind::Surge)
}

/// "If its surge cost was paid": the spell, or the permanent it became, was cast by its card's surge way. Reckless
/// Bushwhacker and Crush of Tentacles ask it.
pub fn surged(q: &dyn Query, id: ObjectId) -> bool {
    let object = q.state().obj(id);
    object.cast_way > 0 && q.card(object.card).ways.get(object.cast_way as usize - 1).is_some_and(|w| w.kind == WayKind::Surge)
}
