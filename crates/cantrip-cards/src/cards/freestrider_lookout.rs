use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_COMMIT_A_CRIME;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "Look at the top five cards of your library. You may put a land card from among them onto the battlefield tapped. Put the
/// rest on the bottom of your library in a random order."
fn look_for_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top_onto_battlefield_select(g, item.controller, item.object, 5, select::LAND_CARD, true, library::Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // Reach is a core keyword. "This ability triggers only once each turn" (CR 603.4a).
    abilities: &[triggered(
        YOU_COMMIT_A_CRIME,
        look_for_a_land,
        "Whenever you commit a crime, look at the top five cards of your library. You may put a land card from among them onto the battlefield tapped. Put the rest on the bottom of your library in a random order. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::FREESTRIDER_LOOKOUT
};
