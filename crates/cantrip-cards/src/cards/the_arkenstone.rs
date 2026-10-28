use cantrip_api::*;
use cantrip_mechanics::statics::CREATURES_YOU_CONTROL_GET_PLUS_ONE;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, library, select};
use cantrip_oracle as oracle;

/// "A legendary creature card" (CR 205.4), as it is in the library.
fn legendary_creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).is(CardType::Creature) && q.chars(card).supertypes.legendary)
}

const LEGENDARY_CREATURE_CARD: Selection = Selection::of(select::up_to(1), &[select::part(&(legendary_creature as SelectFn), 0, "")]).called("legendary creature");

/// "Search your library for a legendary creature card, reveal it, put it into your hand, then shuffle." (CR 701.23): the card
/// need not be found (CR 701.23b); the library is shuffled either way.
fn seek_the_heart(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, LEGENDARY_CREATURE_CARD, true)?;
    Ok(())
}

/// The Adventure (CR 715).
pub static SEEK_THE_HEART: CardDef = CardDef { spell: spell(&[], seek_the_heart), ..oracle::SEEK_THE_HEART };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOUR_END_STEP, |g, item| effects::draw(g, item, 1), "At the beginning of your end step, draw a card.")],
    statics: &[CREATURES_YOU_CONTROL_GET_PLUS_ONE],
    adventure: Some(&SEEK_THE_HEART),
    ..oracle::THE_ARKENSTONE
};

pub static EXTRA: &[&CardDef] = &[&SEEK_THE_HEART];
