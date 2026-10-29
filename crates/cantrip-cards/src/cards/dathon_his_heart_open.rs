use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Dathon adds one more counter of a kind already on a target enchantment he controls (CR 122.1, 608.2d).
fn add_another_counter_of_a_kind(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(enchantment) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(kind) = g.choose_counter_kind(item.controller, item.object, enchantment)? else { return Ok(()) };
    g.add_counters(enchantment, kind, 1);
    Ok(())
}

fn enchantment_you_control(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    matches!(target, Target::Object(id) if q.is_on_battlefield(id)
        && q.controller(id) == asker.controller
        && q.chars(id).is(CardType::Enchantment))
}

/// "Whenever you cast an enchantment spell" (CR 601.2i, 603.2): the spell is the trigger's subject, so its Saga subtype is
/// read from the spell on the stack as the trigger resolves.
fn you_cast_an_enchantment(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == me.controller && q.chars(*spell).is(CardType::Enchantment) => Fired::about(Target::Object(*spell)),
        _ => None,
    }
}

/// Draw for the cast; if that spell was a Saga, Dathon deals 2 damage to each opponent simultaneously (CR 120.2, 101.4).
fn draw_and_damage_sagas(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    if let Target::Object(spell) = item.subject
        && g.chars(spell).has_subtype(g.words(item.object).subtype(Subtype::Saga))
    {
        effects::damage_each_opponent(g, item, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_an_enchantment),
        draw_and_damage_sagas,
        "Whenever you cast an enchantment spell, draw a card. If it's a Saga spell, Dathon deals 2 damage to each opponent.",
    ), ability(cost("{2}"), add_another_counter_of_a_kind, "{2}: Choose a counter on target enchantment you control. Put another counter of that kind on it. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[target(enchantment_you_control)])],
    ..oracle::DATHON_HIS_HEART_OPEN
};
