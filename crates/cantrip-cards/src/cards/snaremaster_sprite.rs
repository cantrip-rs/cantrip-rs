use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to tap target creature an opponent controls and put a stun counter on it?", "", "Pay {cost}", "Don't pay");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{2}"), &PAY),
        "When this creature enters, you may pay {2}. When you do, tap target creature an opponent controls and put a stun counter on it. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
    )],
    delayed: &[delayed_trigger(effects::tap_and_stun::<1>, "When you do, tap target creature an opponent controls and put a stun counter on it.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::SNAREMASTER_SPRITE
};
