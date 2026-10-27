//! Assist (CR 702.132a): "If the total cost to cast a spell with assist includes a generic mana component, before you
//! activate mana abilities while casting it, you may choose another player. That player has a chance to activate mana
//! abilities. Once that player chooses not to activate any more mana abilities, you have a chance to activate mana
//! abilities. Before you begin to pay the total cost of the spell, the player you chose may pay for any amount of the
//! generic mana in the spell's total cost." Charging Binox and Magma Hellion print it.
//!
//! Assist changes how a spell's cost is paid (CR 601.2g-h), so it is the core's (`actions::assist`): a card has it by its
//! content's `"assist": true` (`PayWith::assist`, as convoke, delve and improvise are), which `new-card.py` drafts from the
//! keyword. The cast is offered where the caster's mana and the most another player could pay of the generic mana cover
//! it; the caster is asked whether to choose a player (`Purpose::Assist`), then that player and the caster each get a window
//! for mana abilities (`AssistStep::HelperActivates`, `CasterActivates`), then that player announces how much of the generic
//! mana they pay and pays it from their pool (`HelperAnnounces`, `HelperPays`), and the caster pays the rest with no more
//! mana abilities (`CasterPays`, CR 601.2g). Only generic mana: the colored mana is the caster's (the ruling). A card with
//! nothing else is its generated constant:
//!
//! ```ignore
//! pub static CARD: CardDef = CardDef { ..oracle::CHARGING_BINOX };
//! ```

use cantrip_api::*;

/// Assist as a way of paying, for a card that has it some way its content can't say.
pub const PAY_WITH: PayWith = PayWith { assist: true, ..PayWith::NONE };
