use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The first ability taps the target (CR 701.26a) and puts the stun counter on it whether or not it was untapped (CR 122.1d);
/// tapping it is an effect of this enchantment's controller, so the second ability sees it too and a creature already tapped is
/// no trigger. The second ability is one trigger for each creature tapped (CR 603.2c): its target is chosen as it goes on the
/// stack, so several taps at once put a +1/+1 counter on a creature each, the same one or different ones.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::tap_and_stun::<1>,
            "When this enchantment enters, tap target creature an opponent controls and put a stun counter on it.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS,
            effects::plus_one_counter_on_target,
            "Whenever you tap an untapped creature an opponent controls, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::SOLITARY_SANCTUARY
};
