use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_hand};
use cantrip_mechanics::select::{self, up_to};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for an artifact card with mana value 4 or 5?", "Reveal it and put it into your hand, then shuffle", "Search", "Don't search");

/// "With mana value 4 or 5" (CR 202.3), each card's read where it is (an {X} in a library is 0, CR 202.3e).
fn mana_value_four_or_five(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| (4..=5).contains(&q.mana_value(card)))
}

/// You may search your library for an artifact card with mana value 4 or 5, reveal it, put it into your hand, then shuffle
/// (CR 701.23). The "may" is asked first; declining skips the search and the shuffle.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if !ask_may_search(g, player, item.object, &ASK)? {
        return Ok(());
    }
    let rule = select::artifact_cards(up_to(1)).with(select::part(&(mana_value_four_or_five as SelectFn), 0, "with mana value 4 or 5"));
    find_to_hand(g, player, item.object, rule, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, search, "When this creature enters, you may search your library for an artifact card with mana value 4 or 5, reveal it, put it into your hand, then shuffle.")],
    ..oracle::TRANSIT_MAGE
};
