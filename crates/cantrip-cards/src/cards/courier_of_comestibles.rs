use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_hand};
use cantrip_mechanics::select::{self, Part, Selection};
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

fn food(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).has_subtype(Subtype::Food))
}

/// "A Food card" (CR 205.3g): an artifact card with the subtype Food, a token or not, as it is in the library.
const FOOD_CARD: Part = select::part(&(food as SelectFn), 0, "");

const ASK: Question = Question::new("SearchLibrary", "search your library for a Food card?", "Reveal it and put it into your hand, then shuffle", "Search", "Don't search");

/// "When this creature enters, you may search your library for a Food card, reveal it, put it into your hand, then shuffle.
/// If you don't put a card into your hand this way, create a Food token." (CR 701.23, 111.10b): asked first, and a declined
/// search shuffles nothing; a search that finds no Food card, or whose Food card the player leaves, puts nothing into the
/// hand, so the Food token is made.
fn search_or_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let found = if ask_may_search(g, item.controller, item.object, &ASK)? {
        find_to_hand(g, item.controller, item.object, Selection::of(select::up_to(1), &[FOOD_CARD]).called("Food"), true)?
    } else {
        Vec::new()
    };
    if found.is_empty() {
        tokens::create_food(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search_or_food,
        "When this creature enters, you may search your library for a Food card, reveal it, put it into your hand, then shuffle. If you don't put a card into your hand this way, create a Food token.",
    )],
    ..oracle::COURIER_OF_COMESTIBLES
};
