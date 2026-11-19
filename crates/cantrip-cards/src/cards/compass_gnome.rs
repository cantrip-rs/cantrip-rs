use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_top};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::select;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card or Cave card?",
    "Reveal it, then shuffle and put that card on top of your library",
    "Search",
    "Don't search",
);

/// "When this creature enters, you may search your library for a basic land card or Cave card, reveal it, then shuffle and put
/// that card on top" (CR 701.23): asked first, and a declined search shuffles nothing.
fn search_for_a_basic_land_or_cave_to_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may_search(g, item.controller, item.object, &ASK)? {
        find_to_top(g, item.controller, item.object, select::BASIC_LAND_OR_CAVE_CARD, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search_for_a_basic_land_or_cave_to_top,
        "When this creature enters, you may search your library for a basic land card or Cave card, reveal it, then shuffle and put that card on top.",
    )],
    ..oracle::COMPASS_GNOME
};
