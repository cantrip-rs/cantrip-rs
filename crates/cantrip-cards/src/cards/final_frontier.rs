use cantrip_api::*;
use cantrip_mechanics::{beam_me_up, library, select};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card or Planet card, put it onto the battlefield tapped, then shuffle."
/// The found card enters tapped before the library is shuffled (CR 701.23, 701.24).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::BASIC_LAND_OR_PLANET_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[beam_me_up::way(mana("{1}{G}"), "Beam me up {1}{G}")],
    ..oracle::FINAL_FRONTIER
};
