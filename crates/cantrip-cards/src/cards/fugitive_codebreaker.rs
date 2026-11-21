use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, disguise, effects, triggers};
use cantrip_oracle as oracle;

/// Prowess; Disguise {5}{R}, "this cost is reduced by {1} for each instant and sorcery card in your graveyard" (CR 702.168a, 118.7):
/// the reduction is the card's own, so it applies to the cost of turning it face up (`WayDef::turn_up_change`), not to casting it
/// face down ({3}) or face up ({1}{R}).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(
            triggers::THIS_TURNED_FACE_UP,
            |g, item| effects::discard_hand_then_draw(g, item, 3),
            "When this creature is turned face up, discard your hand, then draw three cards.",
        ),
    ],
    ways: &[disguise::way(
        Cost::mana(mana("{5}{R}")),
        "Disguise {5}{R}. This cost is reduced by {1} for each instant and sorcery card in your graveyard.",
    )
    .turn_up_change(cost_reduction::for_each_instant_and_sorcery_in_your_graveyard)],
    ..oracle::FUGITIVE_CODEBREAKER
};
