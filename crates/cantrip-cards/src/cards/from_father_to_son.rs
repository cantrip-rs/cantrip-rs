use cantrip_api::*;
use cantrip_mechanics::{flashback, library, select};
use cantrip_oracle as oracle;

/// "Search your library for a Vehicle card, reveal it, and put it into your hand. If this spell was cast from a graveyard, put
/// that card onto the battlefield instead. Then shuffle." The card goes to one place only (CR 701.23, 608.2c), asked of how the
/// spell was cast (CR 601.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.history().cast_of(item.object).is_some_and(|c| c.from == ZoneKind::Graveyard) {
        library::find_to_battlefield(g, item.controller, item.object, select::VEHICLE_CARD, false)?;
    } else {
        library::find_to_hand(g, item.controller, item.object, select::VEHICLE_CARD, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{4}{W}{W}{W}"), "Flashback {4}{W}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)")],
    ..oracle::FROM_FATHER_TO_SON
};
