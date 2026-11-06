use cantrip_api::*;
use cantrip_mechanics::COUNTERED_UNLESS_PAID;
use cantrip_oracle as oracle;

/// Check the spell and creatures as the spell is cast (CR 603.2).
fn opposing_unmatched_creature(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = *event else { return None };
    if !q.state().opponents(asker.controller).any(|p| p == player)
        || !q.chars(spell).is(CardType::Creature)
        || q.state().battlefield().iter().copied().any(|id| {
            q.controller(id) == asker.controller && q.chars(id).is(CardType::Creature)
                && q.chars(id).colors.intersects(q.chars(spell).colors)
        })
    {
        return None;
    }
    Some(Fired { subject: Target::Object(spell), data: [u32::from(player.0), u32::MAX] })
}

/// The caster may pay the spell's mana value; otherwise counter it (CR 118.12a, 701.6a).
fn counter_unless_paid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    let price = generic(g.mana_value(spell));
    if !g.may_pay(PlayerId(item.data[0] as u8), item.object, spell, Cost::mana(price), &COUNTERED_UNLESS_PAID)? {
        g.counter(spell);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opposing_unmatched_creature),
        counter_unless_paid,
        "Whenever an opponent casts a creature spell that doesn't share a color with a creature you control, counter that spell unless that player pays {X}, where X is its mana value.",
    )],
    ..oracle::INVOKE_PREJUDICE
};
