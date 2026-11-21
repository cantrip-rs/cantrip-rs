use cantrip_api::*;
use cantrip_mechanics::{lands, library, select, tokens, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card?",
    "The land found enters tapped  ·  searching puts this creature on the bottom of its owner's library, then shuffles, even if no land is found",
    "Search",
    "Don't search",
);

/// "Whenever you sacrifice a Food, you may search your library for a basic land card and put it onto the battlefield tapped. If
/// you search your library this way, put this creature on the bottom of its owner's library, then shuffle." (CR 701.23, 701.24,
/// 608.2c): the search is optional and asked first; the shuffle comes after the creature is on the bottom, not before it, so the
/// creature is shuffled away with the rest. Searching and finding no basic land still counts as searching. A creature that has
/// already left the battlefield is not put anywhere (CR 400.7), and the library is shuffled all the same.
fn search_and_go_to_the_bottom(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (you, source) = (item.controller, item.object);
    if !library::ask_may_search(g, you, source, &ASK)? {
        return Ok(());
    }
    let found = library::search(g, you, you, source, SearchDestination::BattlefieldTapped, select::BASIC_LAND_CARD)?;
    g.put_all_onto_battlefield_as(&found.into_iter().map(|card| (card, you, true)).collect::<Vec<_>>())?;
    g.put_into_library(&[source], LibraryPlace::Bottom);
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Food) }>),
            search_and_go_to_the_bottom,
            "Whenever you sacrifice a Food, you may search your library for a basic land card and put it onto the battlefield tapped. If you search your library this way, put this creature on the bottom of its owner's library, then shuffle.",
        )
        .on_sacrifice(),
    ],
    ..oracle::UNLUCKY_CABBAGE_MERCHANT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
