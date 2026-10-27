//! Freerunning [cost] (CR 702.173a): "You may pay [cost] rather than pay this spell's mana cost if a player was dealt combat
//! damage this turn by a creature that, at the time it dealt that damage, was an Assassin creature or a commander under
//! your control." Brotherhood Ambushers prints it.
//!
//! A card with freerunning lists `way(cost, text)` among its `ways`: an alternative cost (CR 118.9) paid following CR 601.2b
//! and 601.2f-h. The condition is asked of the history: each combat damage dealt to a player this turn with its source as it
//! was when it dealt it (`history::DamageDealt::chars`, the last-known row the damage event names) and its controller then -
//! "it doesn't matter what happens to that Assassin or commander after that" (the ruling). Cantrip's games have no
//! commanders (CR 903.1, 903.3: only the Commander variant designates one, and the engine
//! has no such designation), so in every game it plays no
//! creature is a commander, and only an Assassin creature's damage counts.
//!
//! ```ignore
//! ways: &[freerunning::way(mana("{3}{B}"), "Freerunning {3}{B}")],
//! ```

use cantrip_api::*;

/// Whether a player was dealt combat damage this turn by a creature that was then an Assassin creature its asker controlled
/// (CR 702.173a).
pub fn assassin_dealt_combat_damage(q: &dyn Query, asker: &Asker) -> bool {
    q.history().damage_dealt().any(|d| {
        d.combat && matches!(d.target, Target::Player(_)) && d.controller == asker.controller && {
            let then = d.chars(q);
            then.is(CardType::Creature) && then.has_subtype(Subtype::Assassin)
        }
    })
}

/// The alternative cost of freerunning: `cost` rather than the mana cost (CR 118.9), if an Assassin creature you controlled
/// dealt combat damage to a player this turn. `text` is the keyword as printed ("Freerunning {3}{B}").
pub const fn way(cost: ManaCost, text: &'static str) -> WayDef {
    cantrip_api::way(Cost::mana(cost), ZoneKind::Hand, text).condition(assassin_dealt_combat_damage).kind(WayKind::Freerunning)
}
