use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_TURNED_FACE_UP;
use cantrip_mechanics::{disguise, library};
use cantrip_oracle as oracle;

/// "When this creature is turned face up, look at the top two cards of your library. Put one into your hand and the other into your
/// graveyard" (CR 701.20a, 702.168): with fewer than two cards the one there goes to the hand. Disguise {1}{U/B}{U/B} is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_TURNED_FACE_UP,
        |g, item| library::look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 2),
        "When this creature is turned face up, look at the top two cards of your library. Put one into your hand and the other into your graveyard.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{1}{U/B}{U/B}")), "Disguise {1}{U/B}{U/B}")],
    ..oracle::FAERIE_SNOOP
};
