use cantrip_api::*;
use cantrip_mechanics::{behold, lands};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. You may behold an Elf. If you
/// do, untap that land." (CR 701.4a, 701.23.) The beholding is asked whether or not a land was found; there is only a land
/// to untap if one arrived.
fn fetch_and_maybe_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let arrived = lands::fetch_basic_land_to_battlefield(g, item.controller, item.object, true)?;
    if behold::on_resolution(g, item.controller, item.object, behold::elf, "Elf")? {
        for land in arrived {
            g.untap(land);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(life(1)).and(SACRIFICE_THIS),
        fetch_and_maybe_untap,
        "{T}, Pay 1 life, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. You may behold an Elf. If you do, untap that land.",
    )],
    ..oracle::ELVEN_PASSAGE
};
