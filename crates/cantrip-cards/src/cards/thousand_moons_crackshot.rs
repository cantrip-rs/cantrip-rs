use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to tap target creature?", "", "Pay {cost}", "Don't pay");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{2}{W}"), &PAY),
        "Whenever this creature attacks, you may pay {2}{W}. When you do, tap target creature.",
    )],
    delayed: &[delayed_trigger(effects::tap_targets, "When you do, tap target creature.").targets(&[targets::CREATURE])],
    ..oracle::THOUSAND_MOONS_CRACKSHOT
};
