use cantrip_api::*;
use cantrip_mechanics::{library, select, triggers};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card or Cave card, put it onto the battlefield tapped, then shuffle" (CR 701.23): the
/// searcher finds one matching card or none; the opponent's library is not part of the search.
fn enters_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::BASIC_LAND_OR_CAVE_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        enters_search,
        "When this creature enters, search your library for a basic land card or Cave card, put it onto the battlefield tapped, then shuffle.",
    )],
    ..oracle::SCAMPERING_SURVEYOR
};
