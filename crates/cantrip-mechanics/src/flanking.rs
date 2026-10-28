//! Flanking (CR 702.25a), a triggered ability: "Whenever this creature becomes blocked by a creature without flanking, the
//! blocking creature gets -1/-1 until end of turn." Each instance triggers separately (CR 702.25b). Benalish Cavalry,
//! Jolrael's Centaur, Mtenda Herder, Shadow Rider and Suq'Ata Lancer print it.
//!
//! A card with flanking lists `TRIGGER` among its `abilities`.

use crate::pump;
use cantrip_api::*;

/// Whether the creature has a flanking ability now (CR 702.25a), one it prints or one it was given, and has not lost.
fn has_flanking(q: &dyn Query, creature: ObjectId) -> bool {
    q.chars(creature).abilities().any(|a| q.triggered_ability(a).is_some_and(|t| t.flanking))
}

/// This creature "becomes blocked by a creature without flanking" (CR 509.3d, 603.2): once for each blocker, the blocker
/// the trigger's subject. A blocker with flanking is judged as it blocks, and triggers nothing.
fn blocked_by_a_creature_without_flanking(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Blocks { blocker, attacker, .. } if *attacker == source && !has_flanking(q, *blocker) => Fired::about(Target::Object(*blocker)),
        _ => None,
    }
}

/// "The blocking creature gets -1/-1 until end of turn" (CR 611.2a).
fn blocker_gets_minus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(blocker) = item.subject {
        pump(g, item.object, blocker, -1, -1);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(
    when(EventMask::BLOCKS, blocked_by_a_creature_without_flanking),
    blocker_gets_minus_one,
    "Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)",
)
.flanking();
