use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn, if you control a creature with power 4 or greater" (CR 603.4): an intervening "if",
/// asked as combat begins and again as the ability resolves, the Ranger's own power counting (it is 2/2). "You may discard a card. If
/// you do, draw a card." is chosen as it resolves, so a player with an empty hand is asked nothing.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        |g, item| effects::may_discard_to_draw(g, item, 1),
        "At the beginning of combat on your turn, if you control a creature with power 4 or greater, you may discard a card. If you do, draw a card.",
    )
    .condition(ferocious::you_control_asked)],
    ..oracle::BOUNDARY_LANDS_RANGER
};
