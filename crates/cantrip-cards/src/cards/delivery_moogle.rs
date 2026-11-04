use cantrip_api::*;
use cantrip_mechanics::library::search_zone_and_or_library;
use cantrip_mechanics::{select, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library and graveyard for an artifact card with mana value 2 or less?",
    "Reveal it and put it into your hand, then shuffle. If you don't, you search only your graveyard and must take a card that fits",
    "Search both",
    "Graveyard only",
);

/// "When this creature enters, search your library and/or graveyard for an artifact card with mana value 2 or less, reveal
/// it, and put it into your hand. If you search your library this way, shuffle." (CR 701.23, 701.20a)
///
/// `library::search_zone_and_or_library` with the graveyard: the one card is chosen among both zones, or left unfound in the
/// library (CR 701.23b); a graveyard-only search is no library search (no shuffle) and, the graveyard being public, finds a
/// card that fits.
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let rule = select::artifact_cards(select::up_to(1)).with(select::mana_value_at_most(2));
    let (found, searched_library) =
        search_zone_and_or_library(g, you, item.object, Zone::graveyard(you), SearchDestination::Hand, Purpose::PutIntoHand, &ASK, rule)?;
    let moves: Vec<(ObjectId, Zone)> = found.iter().map(|&c| (c, Zone::hand(you))).collect();
    g.move_together_revealed(&moves, MoveReason::Search);
    if searched_library {
        g.shuffle(you);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        enters,
        "When this creature enters, search your library and/or graveyard for an artifact card with mana value 2 or less, reveal it, and put it into your hand. If you search your library this way, shuffle.",
    )],
    ..oracle::DELIVERY_MOOGLE
};
