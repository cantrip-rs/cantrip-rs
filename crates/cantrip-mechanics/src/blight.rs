//! Blight (CR 701.68a): "to blight N means to put N -1/-1 counters on a creature you control", as a cost. A spell's additional
//! cost ("as an additional cost to cast this spell, you may blight 1": Requiting Hex, Pyrrhic Strike; Cinder Strike and Burning
//! Curiosity print it as an optional cost, and Wild Unraveling and Bogslither's Embrace as one of two ways to pay), or a part of
//! an activated ability's cost ("{T}, Blight 1: Surveil 1": Dawnhand Dissident; Gristle Glutton and Sting-Slinger print it too).
//! Rule 701.68d ("whenever a player blights") has no event yet.
//!
//! Choosing the creature and putting the counters is the core's (`Cost::blight`, CR 601.2h by way of 602.2b for an ability); a
//! player who controls no creature can't choose to blight (CR 701.68b). A card with the optional kind gives `optional(n)` as its
//! `optional_cost`, and "if this spell's additional cost was paid" is `blighted` of the resolving spell; an ability puts
//! `cost(n)` into its cost with `.and`.

use cantrip_api::*;

/// The cost "blight N" as a part of a cost.
pub const fn cost(n: u8) -> Cost {
    Cost { blight: n, ..Cost::NONE }
}

/// "As an additional cost to cast this spell, you may blight N."
pub const fn optional(n: u8) -> OptionalCost {
    OptionalCost::new(cost(n))
}

/// "If this spell's additional cost was paid": it was cast with its optional cost paid. A copy of the spell has it paid
/// when the original did (CR 707.10).
pub fn blighted(item: &StackItem) -> bool {
    item.paid > 0
}
