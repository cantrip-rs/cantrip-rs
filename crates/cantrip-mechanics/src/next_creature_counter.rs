//! "When you next cast a creature spell this turn, that creature enters with an additional +1/+1 counter on it."
//! (CR 603.7b, 614.1c, 122.6), the Saga chapter Summon: Fenrir (II) and Kumano Faces Kakkazan (II) print.
//!
//! The chapter sets up a delayed triggered ability that triggers once, on the first creature spell its controller casts
//! (not a copy, CR 707.10) this turn, read as the spell stands on the stack, and ends with the turn if none is cast. The
//! spell rides along as the trigger's subject, and the trigger's effect makes a replacement effect about it for the turn,
//! applied as it enters the battlefield from the stack (CR 614.12). A card lists `DELAYED` as its delayed ability 0 and
//! `ENTERS_WITH_A_COUNTER` as its granted replacement 0, and its chapter calls `chapter`.

use cantrip_api::*;

/// The chapter's effect: set up the delayed trigger.
pub fn chapter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

/// The delayed trigger's effect, shared by every card whose trigger is about a creature spell as it is cast: it makes the
/// replacement effect about that spell (the trigger's subject) for the turn, the card's granted replacement 0, applied as the
/// spell enters the battlefield from the stack (CR 614.12). Wildgrowth Archaic and Torgal, A Fine Hound give it too.
pub fn give_the_spell_a_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        g.add_effect(item.object, spell, EffectKind::Replacement { card: item.ability.card, index: 0, to: Target::None, into: Target::None, value: 0 }, Duration::EndOfTurn);
    }
    Ok(())
}

/// The delayed trigger, the card's delayed ability 0.
pub const DELAYED: AbilityDef = triggered(
    crate::triggers::CREATURE_SPELL_YOU_CAST,
    give_the_spell_a_counter,
    "When you next cast a creature spell this turn, that creature enters with an additional +1/+1 counter on it.",
);

/// The replacement's condition: the creature spell it is about becomes a permanent (CR 614.12), shared by every card that gives one.
pub fn spell_enters(_: &dyn Query, _: &Asker, affected: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.object == affected && z.from.is(ZoneKind::Stack) && z.to.is(ZoneKind::Battlefield))
}

fn add_a_counter(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += 1;
    }
}

/// The replacement effect the trigger makes about the spell, the card's granted replacement 0.
pub const ENTERS_WITH_A_COUNTER: GrantedReplacementDef =
    granted_replacement(ReplacementKind::ZoneChange, spell_enters, add_a_counter, "enters with an additional +1/+1 counter");
