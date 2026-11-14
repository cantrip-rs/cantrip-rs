use cantrip_api::*;
use cantrip_mechanics::{delirium, library, select};
use cantrip_oracle as oracle;

/// "A Demon card" (CR 205.3, 701.23b): the library may be searched and no card found.
const DEMON_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Demon)]).called("Demon");

/// "Search your library for a Demon card, reveal it, put it into your hand, then shuffle. Delirium - If there are four or
/// more card types among cards in your graveyard, instead search your library for any card, put it into your hand, then
/// shuffle" (CR 207.2c, 701.23): the graveyard is read as the spell resolves, and the one search it makes is the Demon card
/// (revealed) or the any card (not revealed), never both.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if delirium(g, item.controller) {
        library::find_to_hand(g, item.controller, item.object, select::CARD, false)?;
    } else {
        library::find_to_hand(g, item.controller, item.object, DEMON_CARD, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ..oracle::DEMONIC_COUNSEL
};
