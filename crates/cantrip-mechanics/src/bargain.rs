//! Bargain (CR 702.166a): "As an additional cost to cast this spell, you may sacrifice an artifact, enchantment or
//! token." Beseech the Mirror prints it.
//!
//! A card with bargain gives `OPTIONAL_COST` as its `optional_cost` and `LABELS` as its `labels`; "if this spell was
//! bargained" is `bargained` of the resolving spell (CR 702.166b).

use cantrip_api::*;

fn artifact_enchantment_or_token(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    let chars = q.chars(id);
    chars.is(CardType::Artifact) || chars.is(CardType::Enchantment) || q.obj(id).token
}

/// The additional cost: sacrifice an artifact, enchantment or token.
pub const OPTIONAL_COST: OptionalCost = OptionalCost::new(Cost::sacrifice(1, artifact_enchantment_or_token));

pub const LABELS: &[&str] = &["Cast without bargain", "Bargain: Sacrifice an artifact, enchantment, or token"];

/// "If this spell was bargained": it was cast with its optional cost paid.
pub fn bargained(item: &StackItem) -> bool {
    item.paid > 0
}
