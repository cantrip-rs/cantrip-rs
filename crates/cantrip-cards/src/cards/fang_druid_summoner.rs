use cantrip_api::select::part;
use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::library::search_zone_and_or_library_to_hand;
use cantrip_mechanics::select::{self, Part};
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

const MAY_SEARCH: Question = Question::new(
    "SearchLibrary",
    "search your library and/or graveyard for a creature card with no abilities?",
    "Reveal it and put it into your hand",
    "Search",
    "Don't search",
);

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library and graveyard for a creature card with no abilities?",
    "Reveal it and put it into your hand, then shuffle. If you don't, you search only your graveyard and must take a card that fits",
    "Search both",
    "Graveyard only",
);

/// "With no abilities" (CR 113.1, 113.3): `targets::has_no_abilities`, judged where the card is.
fn no_abilities(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| targets::has_no_abilities(q, card))
}
const NO_ABILITIES: Part = part(&(no_abilities as SelectFn), 0, "with no abilities");

/// "When this creature enters, you may search your library and/or graveyard for a creature card with no abilities, reveal it, and
/// put it into your hand. If you search your library this way, shuffle." (CR 701.23, 701.20a, 701.24): the "may" is asked
/// first, as a plain "may" (the graveyard can be searched even when the library can't be, CR 101.2: Ashiok, Dream Render), and
/// the search is then `library::search_zone_and_or_library_to_hand` with the graveyard (the library is searched only if its
/// owner chooses to and can, and then shuffled; a graveyard-only search shuffles nothing and, the graveyard being public, finds
/// a card that fits).
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if !ask_may(g, you, item.object, Purpose::Ask(&MAY_SEARCH))? {
        return Ok(());
    }
    let rule = select::creature_cards(select::up_to(1)).with(NO_ABILITIES);
    search_zone_and_or_library_to_hand(g, you, item.object, &[Zone::graveyard(you)], &ASK, rule)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        enters,
        "When this creature enters, you may search your library and/or graveyard for a creature card with no abilities, reveal it, and put it into your hand. If you search your library this way, shuffle.",
    )],
    ..oracle::FANG_DRUID_SUMMONER
};
