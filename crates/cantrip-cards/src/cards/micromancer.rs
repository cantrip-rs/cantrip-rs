use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_hand};
use cantrip_mechanics::select::{self, up_to};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for an instant or sorcery card with mana value 1?", "Reveal it and put it into your hand, then shuffle", "Search", "Don't search");

/// When this creature enters, you may search your library for an instant or sorcery card with mana value 1, reveal it, put it into
/// your hand, then shuffle (CR 701.23, 202.3). The "may" is asked first; declining skips the search and the shuffle. A card with an
/// {X} in its cost is read with X as 0 in the library (CR 202.3e).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !ask_may_search(g, player, item.object, &ASK)? {
        return Ok(());
    }
    let rule = select::instant_or_sorcery_cards(up_to(1)).with(select::mana_value_exactly(1));
    find_to_hand(g, player, item.object, rule, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search,
        "When this creature enters, you may search your library for an instant or sorcery card with mana value 1, reveal it, put it into your hand, then shuffle.",
    )],
    ..oracle::MICROMANCER
};
