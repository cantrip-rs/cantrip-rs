use cantrip_api::*;
use cantrip_mechanics::library::{self, Rest};
use cantrip_mechanics::select::{self, Part};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "An Ally, Equipment, or Lesson card" (CR 205.3g, 205.3m): a card with any of the three subtypes, as it is in the library.
/// The words are read as Piandao's text has them (CR 612.2).
fn ally_equipment_or_lesson(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let words = q.words(choice.source);
    let (ally, equipment, lesson) = (words.subtype(Subtype::Ally), words.subtype(Subtype::Equipment), words.subtype(Subtype::Lesson));
    Verdict::each(choice, |card| {
        let chars = q.chars(card);
        chars.has_subtype(ally) || chars.has_subtype(equipment) || chars.has_subtype(lesson)
    })
}

const ALLY_EQUIPMENT_OR_LESSON: Part = select::part(&(ally_equipment_or_lesson as SelectFn), 0, "");

/// "Look at the top four cards of your library. You may reveal an Ally, Equipment, or Lesson card from among them and put it into
/// your hand. Put the rest on the bottom of your library in a random order." (CR 701.20a, 401.4)
fn look_at_top_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::cards(select::up_to(1)).with(ALLY_EQUIPMENT_OR_LESSON).called("Ally, Equipment, or Lesson");
    library::look_at_top_select(g, item.controller, item.object, 4, rule, true, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        look_at_top_four,
        "Whenever Master Piandao attacks, look at the top four cards of your library. You may reveal an Ally, Equipment, or Lesson card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::MASTER_PIANDAO
};
