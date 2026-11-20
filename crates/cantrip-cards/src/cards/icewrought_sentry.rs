use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to tap target creature an opponent controls?", "", "Pay {cost}", "Don't pay");

/// The reflexive ability taps its target as an effect of this creature's controller (CR 603.12, 701.26a), so the second ability
/// triggers off it: paying {1}{U} makes this creature 4/4 until end of turn if the target was untapped. Vigilance is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}{U}"), &PAY),
            "Whenever this creature attacks, you may pay {1}{U}. When you do, tap target creature an opponent controls.",
        ),
        triggered(
            YOU_TAP_AN_UNTAPPED_CREATURE_AN_OPPONENT_CONTROLS,
            effects::pump_self::<2, 1>,
            "Whenever you tap an untapped creature an opponent controls, this creature gets +2/+1 until end of turn.",
        ),
    ],
    delayed: &[delayed_trigger(effects::tap_targets, "When you do, tap target creature an opponent controls.").targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::ICEWROUGHT_SENTRY
};
