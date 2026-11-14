use cantrip_api::*;
use cantrip_mechanics::lands::sub;
use cantrip_mechanics::library::{ask_may_search, find_to_graveyard};
use cantrip_mechanics::{combat, select, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for a card?", "Put it into your graveyard, then shuffle", "Search", "Don't search");

/// "You may search your library for a card, put it into your graveyard, then shuffle." (CR 701.23): asked first, so a declined
/// search shuffles nothing; with no quality stated, any card of the library may be found, a land as well as a spell.
fn may_search_to_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may_search(g, item.controller, item.object, &ASK)? {
        find_to_graveyard(g, item.controller, item.object, select::CARD)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    block_rules: &[combat::cant_be_blocked_by_subtype::<{ sub(Subtype::Glimmer) }>],
    abilities: &[triggered(
        triggers::YOUR_SECOND_MAIN_PHASE,
        may_search_to_graveyard,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, you may search your library for a card, put it into your graveyard, then shuffle.",
    )
    .condition(triggers::this_is_tapped)],
    ..oracle::CYNICAL_LONER
};
