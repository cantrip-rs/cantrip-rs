use cantrip_api::*;
use cantrip_mechanics::{flashback, library, select, tokens};
use cantrip_oracle as oracle;

/// "A Mountain card" (CR 205.3i, 305.6): a card with the subtype Mountain, basic or not, the type read through the words of this
/// card's text (CR 612.2) and the card as it is in the library.
fn mountain(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let wanted = q.words(choice.source).subtype(Subtype::Mountain);
    Verdict::each(choice, |card| q.chars(card).has_subtype(wanted))
}

const MOUNTAIN_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::part(&(mountain as SelectFn), 0, "")]).called("Mountain");

/// "Search your library for a Mountain card, reveal it, put it into your hand, then shuffle. Create a 2/2 green Bird creature token
/// with 'Whenever a land you control enters, this token gets +1/+0 until end of turn.'" (CR 701.23, 701.20a, 701.24a): the search
/// may find nothing, the library is shuffled either way, and the token is made all the same.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, MOUNTAIN_CARD, true)?;
    tokens::create_a_landfall_bird(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{5}{R}"), "Flashback {5}{R}")],
    ..oracle::CALL_THE_MOUNTAIN_CHOCOBO
};

pub static EXTRA: &[&CardDef] = &[&tokens::LANDFALL_BIRD];
