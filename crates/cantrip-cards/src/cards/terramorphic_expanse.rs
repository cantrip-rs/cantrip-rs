use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle" (CR 701.23).
fn fetch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_battlefield(g, item.controller, item.object, true)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(SACRIFICE_THIS),
        fetch,
        "{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    )],
    ..oracle::TERRAMORPHIC_EXPANSE
};
