use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "Search your library for a Forest card, put that card onto the battlefield, then shuffle." (CR 701.23, 205.3i): any card with the
/// subtype Forest, basic or not; the player may find none (CR 701.23b), and the library is shuffled either way. The card enters
/// untapped.
fn fetch_a_forest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::FOREST_CARD, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        fetch_a_forest,
        "When this creature enters, search your library for a Forest card, put that card onto the battlefield, then shuffle.",
    )],
    ..oracle::WOOD_ELVES
};
