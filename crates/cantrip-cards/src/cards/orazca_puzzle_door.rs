use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "{1}, {T}, Sacrifice this artifact: Look at the top two cards of your library. Put one of those cards into your hand and the other
/// into your graveyard" (CR 701.20e, 400.3): the sacrifice is a cost (CR 118.3), so the ability resolves even though its source is gone.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP).and(SACRIFICE_THIS),
        |g, item| library::look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 2),
        "{1}, {T}, Sacrifice this artifact: Look at the top two cards of your library. Put one of those cards into your hand and the other into your graveyard.",
    )],
    ..oracle::ORAZCA_PUZZLE_DOOR
};
