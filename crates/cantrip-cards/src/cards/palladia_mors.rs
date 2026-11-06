use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice Palladia-Mors unless you pay {R}{G}{W}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_palladia_mors_unless_pay(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{R}{G}{W}"), &PAY_QUESTION)?;
    g.end_sentence();
    if !paid {
        g.sacrifice(item.controller, &[item.object]);
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
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_palladia_mors_unless_pay,
            "At the beginning of your upkeep, sacrifice Palladia-Mors unless you pay {R}{G}{W}.",
        ),
    ],
    ..oracle::PALLADIA_MORS
};
