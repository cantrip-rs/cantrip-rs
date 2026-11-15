//! Paradigm (CR 702.192a): "If this is the first time a spell you control with this spell's name has resolved this game, at the
//! beginning of each of your precombat main phases for the rest of the game, create a copy of this object in exile. You may
//! cast the copy without paying its mana cost" and "Exile this spell." Germination Practicum, Improvisation Capstone, Restoration Seminar and Decorum Dissertation print it.
//!
//! A card with paradigm sets `paradigm: Some(paradigm::SPELL_ABILITY)`, which the core runs after the card's own effect (and
//! then exiles the spell as it leaves the stack), and lists `paradigm::TRIGGER` first among its `delayed` abilities. The
//! delayed triggered ability is no ability of the card: it is made as the spell resolves and lasts for the rest of the game,
//! whatever happens to the card in exile, and each copy is of the spell as it resolved (the rulings: "Once a spell with
//! paradigm has resolved, it doesn't matter what happens to the card in exile. Even if the card leaves exile, the delayed
//! triggered ability will still trigger during each of your first main phases, and the copy will still be created.").

use cantrip_api::*;

/// "If this is the first time a spell you control with this spell's name has resolved this game", asked as the spell
/// resolves, before its own resolution is recorded: no spell its controller controlled with a name it shares has resolved
/// before, a copy among them (CR 702.192a, 707.10). A spell that was countered or had no legal target did not resolve.
fn is_the_first_to_resolve(q: &dyn Query, item: &StackItem) -> bool {
    !q.history().all().iter().any(|r| {
        matches!(r.event, Event::Resolved { source, ability, .. }
            if ability.is_spell() && q.obj(source).controller == item.controller && crate::same_name(q, source, item.object))
    })
}

/// The spell ability that makes the delayed triggered ability (CR 702.192a, 603.7): "at the beginning of each of your
/// precombat main phases for the rest of the game", controlled by the spell's controller, about the spell as it resolved.
pub fn spell_ability(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_the_first_to_resolve(g, item) {
        let card = g.obj(item.object).card;
        g.delayed_trigger(
            DelayedWhen::EachStep { step: Step::PrecombatMain, on_turn_of: Some(item.controller) },
            item.object,
            item.controller,
            AbilityRef::delayed(card, 0),
            Target::None,
        );
    }
    Ok(())
}

pub const SPELL_ABILITY: Effect = spell_ability;

/// "Create a copy of this object in exile. You may cast the copy without paying its mana cost" (CR 707.12, 118.9): the copy
/// is of the spell as it resolved, wherever its card is now. A copy that could not be cast (no legal target) and a "no"
/// leave it in exile, where it ceases to exist (CR 707.10a); a cast given up the same. A cast copy ceases to exist as it
/// leaves the stack.
fn cast_a_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::copy::may_cast_a_copy_without_paying(g, item.controller, item.object)
}

pub const TRIGGER: AbilityDef = delayed_trigger(
    cast_a_copy,
    "Paradigm: at the beginning of each of your first main phases, you may cast a copy of this spell from exile without paying its mana cost.",
);
