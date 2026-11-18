use cantrip_api::select::part;
use cantrip_api::*;
use cantrip_mechanics::library::{object_names, search_zone_and_or_library_onto_battlefield};
use cantrip_mechanics::select::{self, Part, Selection};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const SEARCH: Question = Question::new(
    "SearchLibrary",
    "search your graveyard, hand, and library for a card named Magnifying Glass and/or a card named Thinking Cap?",
    "What you find, in any of them, goes onto the battlefield, and you shuffle. If you don't, you may still take them from your graveyard and hand",
    "Search all three",
    "Graveyard and hand only",
);

/// "A card named Magnifying Glass" or "a card named Thinking Cap" (CR 201.2a): a card with one of those printed names, as it is in the
/// zone searched. "And/or" takes one of each at most: `DIFFERENT_NAMES` allows no two with the same name.
fn named_magnifying_glass_or_thinking_cap(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| {
        let names = object_names(q, card);
        names.contains(&"Magnifying Glass") || names.contains(&"Thinking Cap")
    })
}

const MAGNIFYING_GLASS_OR_THINKING_CAP: Part = part(&(named_magnifying_glass_or_thinking_cap as SelectFn), 0, "");

/// Up to one card of each name, or none: a search for a quality may find nothing in a hidden zone (CR 701.23b).
const FOUND: Selection =
    Selection::of(select::up_to(2), &[MAGNIFYING_GLASS_OR_THINKING_CAP, select::DIFFERENT_NAMES]).called("card named Magnifying Glass and/or a card named Thinking Cap");

/// "When this creature enters, you may search your graveyard, hand, and/or library for a card named Magnifying Glass and/or a card
/// named Thinking Cap and put them onto the battlefield. If you search your library this way, shuffle." (CR 701.23, 701.24): the
/// "may" is asked first; then the player chooses whether the library is searched too, and the cards found in all the zones
/// searched go onto the battlefield together, the library being shuffled afterwards if it was searched, found or not.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        return Ok(());
    }
    let you = item.controller;
    search_zone_and_or_library_onto_battlefield(g, you, item.object, &[Zone::graveyard(you), Zone::hand(you)], &SEARCH, FOUND)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        search,
        "When this creature enters, you may search your graveyard, hand, and/or library for a card named Magnifying Glass and/or a card named Thinking Cap and put them onto the battlefield. If you search your library this way, shuffle.",
    )],
    ..oracle::AGENCY_OUTFITTER
};
