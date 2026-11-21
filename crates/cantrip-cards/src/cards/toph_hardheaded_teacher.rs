use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, you_cast_a_spell_about_it};
use cantrip_mechanics::{earthbend, effects, targets};
use cantrip_oracle as oracle;

/// "You may discard a card. If you do, return target instant or sorcery card from your graveyard to your hand" (CR 701.9, 608.2c): the
/// target is chosen as the ability goes on the stack, so the card discarded to it can't be that target; the discard is a choice made as
/// it resolves, and the return follows only a card really discarded.
fn discard_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_discard_a_card(g, item)? {
        effects::return_first_target_to_hand(g, item)?;
    }
    Ok(())
}

/// "Earthbend 1. If that spell is a Lesson, put an additional +1/+1 counter on that land" (CR 701.66a, 205.3k): the spell is the trigger's
/// subject, judged as the ability resolves; the land is the one earthbent, still a legal target.
fn earthbend_with_a_lesson_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    earthbend::effect::<1>(g, item)?;
    let Target::Object(spell) = item.subject else { return Ok(()) };
    if g.target_legal(item, 0) && g.chars(spell).has_subtype(Subtype::Lesson) {
        if let Target::Object(land) = item.target(0) {
            g.add_counters(land, CounterKind::PlusOne, 1);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, discard_to_return, "When Toph enters, you may discard a card. If you do, return target instant or sorcery card from your graveyard to your hand.")
            .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD]),
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_spell_about_it),
            earthbend_with_a_lesson_counter,
            "Whenever you cast a spell, earthbend 1. If that spell is a Lesson, put an additional +1/+1 counter on that land. (Target land you control becomes a 0/0 creature with haste that's still a land. Put a +1/+1 counter on it. When it dies or is exiled, return it to the battlefield tapped.)",
        )
        .targets(&[earthbend::TARGET]),
    ],
    delayed: &[earthbend::RETURN],
    ..oracle::TOPH_HARDHEADED_TEACHER
};
