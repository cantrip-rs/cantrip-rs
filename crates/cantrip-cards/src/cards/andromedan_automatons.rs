use cantrip_api::*;
use cantrip_mechanics::select::{self, up_to};
use cantrip_mechanics::{library, triggers, ANY_NUMBER};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for any number of cards named Andromedan Automatons?",
    "Reveal them and put them into your hand, then shuffle",
    "Search",
    "Don't search",
);

/// "You may search your library for any number of cards named Andromedan Automatons, reveal them, put them into your
/// hand, then shuffle" (CR 701.23): asked first; declining skips the search and the shuffle.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !library::ask_may_search(g, player, item.object, &ASK)? {
        return Ok(());
    }
    let rule = select::cards(up_to(ANY_NUMBER)).with(select::named(g.card_id(&CARD)));
    library::find_to_hand(g, player, item.object, rule, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, search, "When this creature enters, you may search your library for any number of cards named Andromedan Automatons, reveal them, put them into your hand, then shuffle.")
    ],
    ..oracle::ANDROMEDAN_AUTOMATONS
};
