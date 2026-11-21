use cantrip_api::*;
use cantrip_mechanics::library::{self, Rest};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// A Lesson, Saga or Shrine (CR 205.3k, 205.3h), the three subtypes read through the words of this card's text (CR 612).
fn is_lesson_saga_or_shrine(words: Words, chars: &Chars) -> bool {
    [Subtype::Lesson, Subtype::Saga, Subtype::Shrine].into_iter().any(|kind| chars.has_subtype(words.subtype(kind)))
}

/// "Look at the top five cards of your library. You may reveal a Lesson, Saga, or Shrine card from among them and put it into
/// your hand. Put the rest on the bottom of your library in a random order." (CR 401.4, 701.20a): a library with fewer cards
/// shows what it has (CR 609.3); with no such card among them all go to the bottom.
fn look_for_a_lesson_saga_or_shrine(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let source = item.object;
    let words = g.words(source);
    library::look_at_top(
        g,
        item.controller,
        source,
        5,
        (0, 1),
        true,
        |c| is_lesson_saga_or_shrine(words, c),
        Rest::BottomRandom,
    )?;
    Ok(())
}

/// "Whenever you cast a Lesson, Saga, or Shrine spell" (CR 603.2, 601.2i): its controller's spells only, judged on the spell as
/// it is on the stack, one cast event triggering once whichever of the subtypes it has.
fn you_cast_a_lesson_saga_or_shrine_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && is_lesson_saga_or_shrine(q.words(me.source), &q.chars(*spell)) => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            look_for_a_lesson_saga_or_shrine,
            "When Guru Pathik enters, look at the top five cards of your library. You may reveal a Lesson, Saga, or Shrine card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        ),
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_lesson_saga_or_shrine_spell),
            effects::plus_one_counter_on_target,
            "Whenever you cast a Lesson, Saga, or Shrine spell, put a +1/+1 counter on another target creature you control.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
    ],
    ..oracle::GURU_PATHIK
};
