use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_hand};
use cantrip_mechanics::select::{self, up_to};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for a creature card with mana value 6 or greater?",
    "Reveal it and put it into your hand, then shuffle",
    "Search",
    "Don't search",
);

/// "You may search your library for a creature card with mana value 6 or greater, reveal it, put it into your hand, then
/// shuffle" (CR 701.23, 701.20): the "may" is asked first, and declining skips the search and the shuffle. The card is a
/// creature card whose mana value is 6 or more as it is in the library.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !ask_may_search(g, player, item.object, &ASK)? {
        return Ok(());
    }
    find_to_hand(g, player, item.object, select::creature_cards(up_to(1)).with(select::mana_value_at_least(6)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search,
        "When this creature enters, you may search your library for a creature card with mana value 6 or greater, reveal it, put it into your hand, then shuffle.",
    )],
    ..oracle::FIERCE_EMPATH
};
