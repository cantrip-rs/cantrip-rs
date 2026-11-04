use cantrip_api::*;
use cantrip_mechanics::{rampage, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice Chromium unless you pay {W}{U}{B}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_chromium_unless_pay_w(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{W}{U}{B}"), &PAY_QUESTION)?;
    g.end_sentence();
    if !paid {
        g.sacrifice(item.controller, &[item.object]);
        g.end_sentence();
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        rampage::trigger::<2>("Rampage 2"),
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_chromium_unless_pay_w,
            "At the beginning of your upkeep, sacrifice Chromium unless you pay {W}{U}{B}.",
        ),
    ],
    ..oracle::CHROMIUM
};
