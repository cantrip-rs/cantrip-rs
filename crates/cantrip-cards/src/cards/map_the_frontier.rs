use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "Search your library for up to two basic land cards and/or Desert cards, put them onto the battlefield tapped, then
/// shuffle" (CR 205.3i, 701.23): any mix of the two, the subtype read through the words of this card (CR 612.2).
fn search_for_basic_lands_and_deserts(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::cards(select::up_to(2)).with(select::basic_land_or_subtype(Subtype::Desert)).called("basic land or Desert");
    library::find_to_battlefield(g, item.controller, item.object, rule, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], search_for_basic_lands_and_deserts), ..oracle::MAP_THE_FRONTIER };
