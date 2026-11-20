//! "You may cast this spell as though it had flash if you pay {2} more to cast it" (CR 307.5, 601.2b, 702.8): the card's own
//! permission to be cast whenever its caster could cast an instant, bought with an additional cost the caster chooses to pay
//! as they propose the spell (CR 601.2b, 601.2f, 118.8). It is the card's optional cost (`OptionalCost::flash`), announced
//! before the targets as a kicker is and added to the total with the cost changes (CR 601.2f); the cast owes it only where
//! nothing else lets the spell be cast then, and may pay it anyway (the rulings: Harbinger of the Tides, Oakshade Stalker). The
//! {2} is no part of the mana cost, so the mana value is unchanged (CR 202.3). Harbinger of the Tides, Mystical Tether and
//! Asinine Antics print it.
//!
//! A card gives `OPTIONAL_COST` as its `optional_cost` and `LABELS` as its `labels`, the two answers of the question that
//! offers it.

use cantrip_api::*;

/// "If you pay {2} more to cast it": the additional cost of the permission.
pub const OPTIONAL_COST: OptionalCost = OptionalCost { flash: true, ..OptionalCost::new(Cost::mana(generic(2))) };

/// The two answers of the question that offers the cost: cast without it, or pay it for flash (`OptionalCost::flash`).
pub const LABELS: &[&str] = &["Cast without flash", "Flash: Pay {2} more"];
