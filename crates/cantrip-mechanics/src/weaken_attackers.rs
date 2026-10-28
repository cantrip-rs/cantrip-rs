//! "Until your next turn, whenever a creature attacks you or a planeswalker you control, it gets -N/-0 until end of turn"
//! (CR 603.7, 611.2): a delayed triggered ability that watches every attack on its creator or their planeswalkers until their
//! next turn begins. Tamiyo, Seasoned Scholar (-1/-0) and Jace, Reality Sculptor (-5/-0) print it, each with its own N.

use crate::{pump, triggers};
use cantrip_api::*;

/// The delayed ability a card lists in its `delayed` slot: "whenever a creature attacks you or a planeswalker you control,
/// it gets -`N`/-0 until end of turn".
pub const fn delayed_ability<const N: i16>(text: &'static str) -> AbilityDef {
    triggered(when_asked(EventMask::ATTACKS, triggers::creature_attacks_you_or_a_planeswalker_you_control), attacker_weakens::<N>, text)
}

fn attacker_weakens<const N: i16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        pump(g, item.object, creature, -N, 0);
    }
    Ok(())
}

/// The resolving ability's "until your next turn": creates the delayed ability at index 0 of `def`'s `delayed` slot, which
/// watches until `item.controller`'s next turn (CR 603.7b), "you" being that controller whoever controls the source now.
pub fn until_your_next_turn(g: &mut dyn Game, item: &StackItem, def: &'static CardDef) {
    let card = g.card_id(def);
    g.delayed_trigger(DelayedWhen::UntilNextTurnOf(item.controller), item.object, item.controller, AbilityRef::delayed(card, 0), Target::None);
}
