use cantrip_api::*;
use cantrip_mechanics::triggers::{PROWESS, THIS_ATTACKS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "PayForBlockRestriction",
    "pay {cost} so target creature can't block this turn?",
    "When you do, target creature can't block this turn",
    "Pay {cost}",
    "Don't pay",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        triggered(
            THIS_ATTACKS,
            |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}"), &PAY),
            "Whenever Ty Lee attacks, you may pay {1}. When you do, target creature can't block this turn.",
        ),
    ],
    delayed: &[delayed_trigger(effects::first_target_cant_block_this_turn, "When you do, target creature can't block this turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::TY_LEE_ARTFUL_ACROBAT
};
