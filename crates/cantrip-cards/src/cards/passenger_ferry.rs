use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} so another target attacking creature can't be blocked this turn?", "", "Pay {cost}", "Don't pay");

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{U}"), &PAY),
            "Whenever this Vehicle attacks, you may pay {U}. When you do, another target attacking creature can't be blocked this turn.",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    delayed: &[delayed_trigger(
        effects::target_creature_cant_be_blocked_this_turn,
        "When you do, another target attacking creature can't be blocked this turn.",
    )
    .targets(&[targets::ANOTHER_ATTACKING_CREATURE])],
    ..oracle::PASSENGER_FERRY
};
