use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature spell, if at least four mana was spent to cast it" (CR 603.2, 603.4, 601.2h): the mana spent is
/// the whole of the payment, additional costs too, and is read from the spell's own object as it is cast (a spell cast without
/// spending mana has none). Answered as it triggers; what was spent on the spell does not change afterwards.
fn noncreature_spell_you_cast_with_four_or_more_mana_spent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, .. } if q.obj(*spell).spent.total() >= 4 => cantrip_mechanics::triggers::noncreature_spell_you_cast(q, me, event),
        _ => None,
    }
}

/// "Put a +1/+1 counter on this creature and it can't be blocked this turn" (CR 122.1, 509.1b, 611.2): both only while it is
/// still the creature on the battlefield (CR 400.7); it resolves whether or not the spell that caused it still exists.
fn grow_and_cant_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
        effects::this_cant_be_blocked_this_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, noncreature_spell_you_cast_with_four_or_more_mana_spent),
        grow_and_cant_be_blocked,
        "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature and it can't be blocked this turn.",
    )],
    ..oracle::SAHAGIN
};
