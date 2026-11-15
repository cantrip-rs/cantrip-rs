//! Forage (CR 701.61a): "to forage means 'exile three cards from your graveyard or sacrifice a Food'", as a cost.
//!
//! The two ways to forage are two ways of paying, so a spell that asks for it as an additional cost lists them beside its other
//! way: Feed the Cycle's "forage or pay {B}" is `additional_choice: &[EXILE_THREE_CARDS, SACRIFICE_A_FOOD, <pay {B}>]`
//! (CR 601.2b: the player chooses one, and only a way that can be paid is offered, CR 118.3). Exiling three cards takes
//! them from the casting player's own graveyard, never the spell itself (it is on the stack, CR 601.2a), and sacrificing a
//! Food takes any Food they control, a Food token or a Food artifact card.

use cantrip_api::*;

use crate::targets::filters;

/// "Exile three cards from your graveyard": the first way to forage (CR 701.61a).
pub const EXILE_THREE_CARDS: ExtraCost = ExtraCost { text: "Forage: exile three cards from your graveyard", cost: exile_from_graveyard(3) };

/// "Sacrifice a Food": the second way to forage (CR 701.61a).
pub const SACRIFICE_A_FOOD: ExtraCost = ExtraCost { text: "Forage: sacrifice a Food", cost: Cost::sacrifice(1, filters::food) };
