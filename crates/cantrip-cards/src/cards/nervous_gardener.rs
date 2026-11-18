use cantrip_api::*;
use cantrip_mechanics::library::find_to_hand;
use cantrip_mechanics::select::LAND_WITH_A_BASIC_LAND_TYPE_CARD;
use cantrip_mechanics::{disguise, triggers};
use cantrip_oracle as oracle;

/// "Search your library for a land card with a basic land type, reveal it, put it into your hand, then shuffle" (CR 701.23): not
/// optional, though the player may find nothing, and the library is shuffled either way.
fn search_for_a_land_with_a_basic_land_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    find_to_hand(g, item.controller, item.object, LAND_WITH_A_BASIC_LAND_TYPE_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{G}")), "Disguise {G}")],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        search_for_a_land_with_a_basic_land_type,
        "When this creature is turned face up, search your library for a land card with a basic land type, reveal it, put it into your hand, then shuffle.",
    )],
    ..oracle::NERVOUS_GARDENER
};
