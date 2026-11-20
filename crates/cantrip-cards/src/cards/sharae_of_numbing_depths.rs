use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever you tap one or more untapped creatures your opponents control, draw a card. This ability triggers only once each
/// turn." (CR 603.2c; "triggers only once each turn" has no numbered rule): the creatures tapped together are one batch and
/// trigger it once, and a second batch the same turn does not trigger it again (`triggers_each_turn(1)`); the first ability's
/// tap and a later "tap all" are different batches.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::tap_and_stun::<1>,
            "When Sharae enters, tap target creature an opponent controls and put a stun counter on it.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS,
            |g, item| effects::draw(g, item, 1),
            "Whenever you tap one or more untapped creatures your opponents control, draw a card. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1)
        .once_per_batch(),
    ],
    ..oracle::SHARAE_OF_NUMBING_DEPTHS
};
