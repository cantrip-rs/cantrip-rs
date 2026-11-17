use cantrip_api::*;
use cantrip_mechanics::statics::flash_if_you_control_a;
use cantrip_mechanics::{lands, library, select};
use cantrip_oracle as oracle;

/// "A Desert card" (CR 205.3i): a card with the subtype Desert.
const DESERT_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Desert)]).called("Desert");

/// "Search your library for a Desert card, put it onto the battlefield tapped, then shuffle" (CR 701.23, 205.3i).
fn search_for_a_desert(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, DESERT_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "You may cast this spell as though it had flash if you control a Desert" (CR 601.3b, 702.8): asked once as casting begins.
    // Trample is a core keyword.
    flash_if: Some(flash_if_you_control_a::<{ lands::sub(Subtype::Desert) }>),
    abilities: &[ability(
        cost("{1}{G}").and(EXILE_THIS),
        search_for_a_desert,
        "{1}{G}, Exile this card from your graveyard: Search your library for a Desert card, put it onto the battlefield tapped, then shuffle.",
    )
    .from(ZoneKind::Graveyard)],
    ..oracle::COLOSSAL_RATTLEWURM
};
