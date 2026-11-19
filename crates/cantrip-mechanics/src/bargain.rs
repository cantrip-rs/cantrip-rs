//! Bargain (CR 702.166a): "As an additional cost to cast this spell, you may sacrifice an artifact, enchantment or
//! token." Beseech the Mirror prints it.
//!
//! A card with bargain gives `OPTIONAL_COST` as its `optional_cost` and `LABELS` as its `labels`; "if this spell was
//! bargained" is `bargained` of the resolving spell (CR 702.166b).

use cantrip_api::*;

fn artifact_enchantment_or_token(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    crate::select::is_artifact_enchantment_or_token(q, id)
}

/// The additional cost: sacrifice an artifact, enchantment or token.
pub const OPTIONAL_COST: OptionalCost = OptionalCost::new(Cost::sacrifice(1, artifact_enchantment_or_token));

pub const LABELS: &[&str] = &["Cast without bargain", "Bargain: Sacrifice an artifact, enchantment, or token"];

/// "If this spell was bargained": it was cast with its optional cost paid.
pub fn bargained(item: &StackItem) -> bool {
    item.paid > 0
}

/// An intervening "if it was bargained" (CR 603.4, 702.166a) as a condition (`AbilityDef::condition`) of a permanent's own
/// triggered ability: the permanent was cast with its optional cost paid (`Obj::paid`, carried from the spell to the permanent),
/// checked as the ability triggers and again as it resolves; one that has left the battlefield is judged by how it last
/// existed there (CR 608.2h). A permanent put onto the battlefield without being cast was never bargained. Agatha's Champion,
/// Diminisher Witch, High Fae Negotiator, Realm-Scorcher Hellkite, Tenacious Tomeseeker and Troublemaker Ouphe print it.
pub fn was_bargained(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).paid > 0
}
