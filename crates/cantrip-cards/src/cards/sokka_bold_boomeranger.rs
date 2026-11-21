use cantrip_api::*;
use cantrip_mechanics::triggers::{subtype_spell_you_cast, artifact_spell_you_cast, THIS_ENTERS};
use cantrip_mechanics::{effects, hand, lands};
use cantrip_oracle as oracle;

/// "When Sokka enters, discard up to two cards, then draw that many cards" (CR 701.9, 121.1): the discards happen together, then
/// the draw of as many as were chosen.
fn discard_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::discard_up_to_then_draw(g, item.controller, item.object, 2)
}

/// "Whenever you cast an artifact or Lesson spell" (CR 603.2, 601.2i): one cast event triggers once, whether the spell is an
/// artifact, a Lesson or both.
fn artifact_or_lesson_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    artifact_spell_you_cast(q, me, event).or_else(|| subtype_spell_you_cast::<{ lands::sub(Subtype::Lesson) }>(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, discard_and_draw, "When Sokka enters, discard up to two cards, then draw that many cards."),
        triggered(
            when_asked(EventMask::SPELL_CAST, artifact_or_lesson_spell_you_cast),
            effects::plus_one_counter_on_this,
            "Whenever you cast an artifact or Lesson spell, put a +1/+1 counter on Sokka.",
        ),
    ],
    ..oracle::SOKKA_BOLD_BOOMERANGER
};
