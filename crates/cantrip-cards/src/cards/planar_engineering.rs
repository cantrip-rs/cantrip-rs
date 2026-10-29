use cantrip_api::*;
use cantrip_mechanics::{library, sacrifice_chosen, select};
use cantrip_oracle as oracle;

/// "Sacrifice two lands. Search your library for four basic land cards, put them onto the battlefield tapped, then
/// shuffle." (CR 701.21a, 701.23): with fewer than two lands you sacrifice those you have, and the search happens either
/// way; it finds up to four basic land cards, fewer if the library has fewer, and the library is shuffled regardless.
fn engineer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    sacrifice_chosen(g, item.controller, item.object, 2, |c| c.is(CardType::Land))?;
    library::find_to_battlefield(g, item.controller, item.object, select::basic_land_cards(select::up_to(4)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], engineer), ..oracle::PLANAR_ENGINEERING };
