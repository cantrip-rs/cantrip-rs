use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::library::{ask_may_search, put_found_into_hand, search};
use cantrip_mechanics::select;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for an artifact card?",
    "Reveal it, put it onto the battlefield if its mana value is 2 or less and into your hand otherwise, then shuffle",
    "Search",
    "Don't search",
);

/// "You may search your library for an artifact card and reveal it. Put it onto the battlefield if its mana value is 2 or less.
/// Otherwise, put it into your hand. If you search your library this way, shuffle." (CR 701.23, 701.20, 202.3): asked first, and a
/// declined search shuffles nothing. The card may be left unfound (CR 701.23b); the library is shuffled whether or not one was.
/// Its mana value is the one it has in the library (an {X} in it is 0, CR 202.3e).
fn search_for_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !ask_may_search(g, player, item.object, &ASK)? {
        return Ok(());
    }
    let found = search(g, player, player, item.object, SearchDestination::Found, select::ARTIFACT_CARD)?;
    g.reveal(player, &found);
    if let Some(&card) = found.first() {
        if g.mana_value(card) <= 2 {
            g.put_onto_battlefield(card, player, false)?;
        } else {
            put_found_into_hand(g, player, &found, true);
        }
    }
    g.shuffle(player);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            search_for_an_artifact,
            "When this Vehicle enters, you may search your library for an artifact card and reveal it. Put it onto the battlefield if its mana value is 2 or less. Otherwise, put it into your hand. If you search your library this way, shuffle.",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::GUIDELIGHT_PATHMAKER
};
