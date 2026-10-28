//! Teamwork (CR 702.194): "Teamwork N" is "as an additional cost to cast this spell, you may tap any number of creatures you
//! control with total power N or more" (702.194a), announced before the targets as a kicker is, and paid with the other costs
//! (CR 601.2b, 601.2f-h). Summoning sickness does not matter, since the creatures' own {T} is not the cost (CR 302.6). A creature
//! tapped for it becomes tapped to pay a teamwork cost: `Event::TappedForTeamwork` (Agent Maria Hill).
//!
//! Printed on We Say Thee Nay!, Helicarrier Strike and Murdock's Crusade. A card gives `optional(n)` as its `optional_cost`,
//! with `labels` for the question that offers it, and "if this spell was cast using teamwork" (702.194b) is `used` of the
//! resolving spell. A part of the spell that has targets only when teamwork was used (702.194c) is a mode chosen by it
//! (`ModalDef::when_paid`).

use cantrip_api::*;

/// "Teamwork N" (CR 702.194a) as the spell's optional additional cost.
pub const fn optional(power: u8) -> OptionalCost {
    OptionalCost { cost: Cost { tap_creatures: Some(TapCreatures { power, greater_with: None }), ..Cost::NONE } }
}

/// "If this spell was cast using teamwork" (CR 702.194b): its caster declared the intention to pay the teamwork cost as they
/// cast it. A copy of the spell was cast the same way when the original was (CR 707.10).
pub fn used(item: &StackItem) -> bool {
    item.paid > 0
}
