use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Reveal cards from the top of your library until you reveal a land card. Put that card onto the battlefield tapped and the rest on
/// the bottom of your library in a random order" (CR 701.20, 401.4): the shared `library::reveal_until_a_land_onto_battlefield_tapped`.
fn reveal_until_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::reveal_until_a_land_onto_battlefield_tapped(g, item.controller)
}

/// Reach is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        reveal_until_a_land,
        "When this creature enters, reveal cards from the top of your library until you reveal a land card. Put that card onto the battlefield tapped and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::CLIFFTOP_LOOKOUT
};
