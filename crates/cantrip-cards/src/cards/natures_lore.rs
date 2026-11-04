use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "Search your library for a Forest card, put that card onto the battlefield, then shuffle." (CR 608.2, 701.24a).
fn search_library_forest_card_put(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::FOREST_CARD, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_library_forest_card_put),
    ..oracle::NATURES_LORE
};
