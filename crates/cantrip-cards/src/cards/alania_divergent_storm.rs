use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{same_name, targets};
use cantrip_oracle as oracle;

/// "Whenever you cast a spell, if it's the first instant spell, the first sorcery spell, or the first Otter spell other than
/// Alania you've cast this turn" (CR 603.2, 603.4): a spell its controller casts that is an instant, a sorcery or an Otter other
/// than Alania, and that no earlier spell of the same kind was cast this turn, each kind judged as the spell was cast (a spell
/// that is an instant and an Otter counts for either), "Otter" a word of Alania's text (CR 612) and an earlier cast of Alania
/// herself not counting among the Otters. The spell rides along as the trigger's subject.
fn first_instant_sorcery_or_otter_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    if *player != me.controller {
        return None;
    }
    let otter = q.words(me.source).subtype(Subtype::Otter);
    let kinds: [&dyn Fn(ObjectId, &Chars) -> bool; 3] = [
        &|_, chars| chars.is(CardType::Instant),
        &|_, chars| chars.is(CardType::Sorcery),
        &|id, chars| !same_name(q, id, me.source) && chars.has_subtype(otter),
    ];
    let first_of_a_kind = kinds.iter().any(|kind| kind(*spell, &q.chars(*spell)) && !q.history().casts_by(*player).any(|c| c.spell != *spell && kind(c.spell, &c.chars(q))));
    if first_of_a_kind { Fired::about(Target::Object(*spell)) } else { None }
}

/// "You may have target opponent draw a card. If you do, copy that spell. You may choose new targets for the copy." (CR 707.10,
/// 707.10c): the opponent must still be a legal target; the copy, made only if the draw was chosen, is the controller's and is not
/// cast, and it goes on the stack above the spell that caused the trigger.
fn opponent_may_draw_then_copy_the_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(opponent), Some(spell)) = (targets::player(g, item, 0), item.subject.object()) else { return Ok(()) };
    if ask_may(g, item.controller, item.object, Purpose::Optional)? {
        g.draw(opponent, 1);
        g.copy_spell(spell, item.controller, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, first_instant_sorcery_or_otter_you_cast),
        opponent_may_draw_then_copy_the_spell,
        "Whenever you cast a spell, if it's the first instant spell, the first sorcery spell, or the first Otter spell other than Alania you've cast this turn, you may have target opponent draw a card. If you do, copy that spell. You may choose new targets for the copy.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::ALANIA_DIVERGENT_STORM
};
