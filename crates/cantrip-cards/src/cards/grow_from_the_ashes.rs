use cantrip_api::*;
use cantrip_mechanics::{lands, library, select};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put it onto the battlefield, then shuffle. If this spell was kicked, instead
/// search your library for two basic land cards, put them onto the battlefield, then shuffle." (CR 701.23, 702.33): one search
/// either way, the lands untapped; fewer may be found, or none (CR 701.23b), and the library is shuffled either way.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.kicked > 0 {
        library::find_to_battlefield(g, item.controller, item.object, select::basic_land_cards(select::up_to(2)), false)?;
        Ok(())
    } else {
        lands::search_basic_land_to_battlefield(g, item.controller, item.object, false)
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search),
    kicker: kicker(mana("{2}")),
    ..oracle::GROW_FROM_THE_ASHES
};
