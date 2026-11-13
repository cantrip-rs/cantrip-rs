use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put that card onto the battlefield, then shuffle." (CR 701.23, 701.24a).
fn search_basic_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_battlefield(g, item.controller, item.object, false)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_basic_land),
    ..oracle::UNTAMED_WILDS
};
