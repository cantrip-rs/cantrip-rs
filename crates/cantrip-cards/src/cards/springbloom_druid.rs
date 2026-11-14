use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "You may sacrifice a land. If you do, search your library for up to two basic land cards, put them onto the battlefield
/// tapped, then shuffle." (CR 701.21a, 701.23, 701.24).
fn sacrifice_a_land_to_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::may_sacrifice_to_find_to_battlefield_tapped(g, item, select::land_cards(select::up_to(1)), select::basic_land_cards(select::up_to(2)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        sacrifice_a_land_to_search,
        "When this creature enters, you may sacrifice a land. If you do, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
    )],
    ..oracle::SPRINGBLOOM_DRUID
};
