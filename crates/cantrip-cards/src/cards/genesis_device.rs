use cantrip_api::*;
use cantrip_mechanics::{effects, library, select, triggers};
use cantrip_oracle as oracle;

/// "When Genesis Device enters, search your library for a basic land card or Planet card, put it onto the battlefield
/// tapped, then shuffle" (CR 701.23, 205.3i, 205.4c): the searcher finds one matching card or none; the opponent's library is not
/// part of the search.
fn enters_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::BASIC_LAND_OR_PLANET_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            enters_search,
            "When Genesis Device enters, search your library for a basic land card or Planet card, put it onto the battlefield tapped, then shuffle.",
        ),
        // "Activate only as a sorcery" (CR 602.5d): the activated ability uses sorcery timing.
        ability(
            cost("{3}{G}{G}, {T}").and(SACRIFICE_THIS),
            effects::sacrifice_any_number_of_lands_for_lands,
            "{3}{G}{G}, {T}, Sacrifice Genesis Device: Sacrifice any number of lands you control, then search your library for up to that many land cards, put them onto the battlefield tapped, then shuffle. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::GENESIS_DEVICE
};
