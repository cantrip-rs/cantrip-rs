//! Increment (CR 702.191a): "Whenever you cast a spell, if this permanent is a creature and the amount of mana spent to cast
//! that spell is greater than this creature's power or toughness, put a +1/+1 counter on this creature." Each instance
//! triggers separately (CR 702.191b). Pensive Professor, Tester of the Tangential and Textbook Tabulator print it.
//!
//! A card with increment lists `TRIGGER` among its `abilities`.

use crate::effects::plus_one_counter_on_this;
use cantrip_api::*;

/// The mana spent on `spell` (CR 601.2h; none for a spell cast without paying) is greater than the power or the toughness of
/// `source`, which must be a creature. The numbers are compared as signed values: a power below zero is beaten by any amount.
fn mana_spent_beats(q: &dyn Query, source: ObjectId, spell: ObjectId) -> bool {
    let chars = q.chars(source);
    if !chars.is(CardType::Creature) {
        return false;
    }
    let spent = i64::from(q.obj(spell).spent.total());
    chars.power.is_some_and(|p| spent > i64::from(p)) || chars.toughness.is_some_and(|t| spent > i64::from(t))
}

/// Checked as the spell is cast, when the ability triggers. A copy of a spell is not cast (CR 707.10), so it triggers nothing.
fn you_cast_a_spell_with_more_mana_than_a_stat(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && mana_spent_beats(q, me.source, *spell) => {
            Fired::about(Target::Object(*spell))
        }
        _ => None,
    }
}

/// The intervening "if" is checked again on resolution (CR 603.4): the permanent must still be a creature on the battlefield,
/// and the comparison is made afresh with its present power and toughness - either may have changed since the trigger. The
/// spell's mana spent is read from its object, so this still answers once the spell has been countered or has left the stack.
fn put_counter_if_still_beaten(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject
        && g.is_on_battlefield(item.object)
        && mana_spent_beats(g, item.object, spell)
    {
        return plus_one_counter_on_this(g, item);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(
    when_asked(EventMask::SPELL_CAST, you_cast_a_spell_with_more_mana_than_a_stat),
    put_counter_if_still_beaten,
    "Increment (Whenever you cast a spell, if the amount of mana you spent is greater than this creature's power or toughness, put a +1/+1 counter on this creature.)",
);
