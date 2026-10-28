//! Increment (CR 702.191a), a triggered ability: "Whenever you cast a spell, if this permanent is a creature and the amount of
//! mana spent to cast that spell is greater than this creature's power or this creature's toughness, put a +1/+1 counter on
//! this creature." Each instance triggers separately (CR 702.191b). Cuboid Colony, Hungry Graffalon, Pensive Professor, Tester of the Tangential and Textbook Tabulator print it.
//!
//! An intervening "if" (CR 603.4): checked as the spell is cast and again as the ability resolves, with the mana spent read from
//! the spell itself (the trigger's subject), so it still answers if the spell was countered or has left the stack (CR 608.2h),
//! and with the creature's power and toughness as they are then - either may be the one that is less.
//!
//! A card with increment lists `TRIGGER` among its `abilities`.

use crate::effects::plus_one_counter_on_this;
use cantrip_api::*;

/// Whether the mana spent on `spell` is greater than `creature`'s power or its toughness, and it is a creature.
fn more_mana_spent_than_power_or_toughness(q: &dyn Query, creature: ObjectId, spell: ObjectId) -> bool {
    let c = q.chars(creature);
    let spent = i64::from(q.obj(spell).spent.total());
    c.is(CardType::Creature) && (c.power.is_some_and(|p| spent > i64::from(p)) || c.toughness.is_some_and(|t| spent > i64::from(t)))
}

fn you_cast_a_spell_costing_more_than_this_creature_is(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && more_mana_spent_than_power_or_toughness(q, me.source, *spell) => {
            Fired::about(Target::Object(*spell))
        }
        _ => None,
    }
}

/// As it resolves: the same question again, about the spell that caused it.
fn put_a_counter_if_still_more(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject
        && more_mana_spent_than_power_or_toughness(g, item.object, spell)
    {
        return plus_one_counter_on_this(g, item);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(
    when_asked(EventMask::SPELL_CAST, you_cast_a_spell_costing_more_than_this_creature_is),
    put_a_counter_if_still_more,
    "Increment (Whenever you cast a spell, if the amount of mana you spent is greater than this creature's power or toughness, put a +1/+1 counter on this creature.)",
);
