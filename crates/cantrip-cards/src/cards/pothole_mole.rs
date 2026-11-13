use cantrip_api::*;
use cantrip_mechanics::{library, select, triggers};
use cantrip_oracle as oracle;

/// "Mill three cards, then you may return a land card from your graveyard to your hand" (CR 701.17a, 608.2c): the choice is
/// made from the whole graveyard as the mill left it, a card just milled included, and the player may choose none.
fn mill_then_return_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_return_a_card(g, item, 3, select::LAND_CARD)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        mill_then_return_a_land,
        "When this creature enters, mill three cards, then you may return a land card from your graveyard to your hand.",
    )],
    ..oracle::POTHOLE_MOLE
};
