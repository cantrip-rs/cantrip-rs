use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, Find, search_to_battlefield};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for up to that many land cards?", "They enter tapped, then you shuffle", "Search", "Don't search");

/// "Whenever this creature is dealt damage, you may search your library for up to that many land cards, put them onto the
/// battlefield tapped, then shuffle." (CR 120.2, 701.23, 614.1c) "That many" is the damage dealt at once, the trigger's
/// `event_amount`, and it triggers even when the damage is lethal: the search is made by the ability's controller after it has
/// gone. Any land cards, not only basic ones. Declining skips the search and the shuffle; a search for up to that many may
/// find fewer (CR 701.23d), and the library is shuffled either way.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !ask_may_search(g, item.controller, item.object, &ASK)? {
        return Ok(());
    }
    let available = g.state().zone(Zone::library(item.controller)).len();
    let count = item.matched_damage_amount().min(available.min(u32::MAX as usize) as u32);
    search_to_battlefield(g, item.controller, item.object, Find::ONE.up_to(count), true, |q, c| q.chars(c).is(CardType::Land))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_TAKES_DAMAGE,
        search,
        "Whenever this creature is dealt damage, you may search your library for up to that many land cards, put them onto the battlefield tapped, then shuffle.",
    ).damage_amount()
    .sums_per_subject()],
    ..oracle::HEXHAVEN_INVIGORATOR
};
