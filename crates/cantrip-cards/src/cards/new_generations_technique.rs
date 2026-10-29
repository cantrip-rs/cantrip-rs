use cantrip_api::*;
use cantrip_mechanics::{library, select, sneak};
use cantrip_oracle as oracle;

/// "Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle" (CR 701.23, 701.24):
/// fewer may be found, or none, and the library is shuffled either way.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::basic_land_cards(select::up_to(2)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search),
    ways: &[sneak::way(mana("{2}{G}"), "Sneak {2}{G}")],
    ..oracle::NEW_GENERATIONS_TECHNIQUE
};
