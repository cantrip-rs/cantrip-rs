use cantrip_api::*;
use cantrip_mechanics::{flashback, library::look_at_top_take_one_rest_to_graveyard};
use cantrip_oracle as oracle;

/// "Look at the top three cards of your library. Put one of them into your hand and the rest into your graveyard." (CR 701.20a,
/// 400.3): `look_at_top_take_one_rest_to_graveyard`, which takes the one card and sends the others together.
fn revelation(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 3)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], revelation),
    ways: &[flashback::way(cost("{6}{B}"), "Flashback {6}{B}")],
    ..oracle::RESENTFUL_REVELATION
};
