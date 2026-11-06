use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice Vaevictis Asmadi unless you pay {B}{R}{G}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_vaevictis_asmadi_unless_pay(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{B}{R}{G}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "If you don't, sacrifice {subject} if able",
    "Pay {cost}",
    "Don't pay",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_vaevictis_asmadi_unless_pay,
            "At the beginning of your upkeep, sacrifice Vaevictis Asmadi unless you pay {B}{R}{G}.",
        ),
        ability(
            cost("{B}"),
            effects::this_gets::<1, 0>,
            "{B}: Vaevictis Asmadi gets +1/+0 until end of turn.",
        ),
        ability(
            cost("{R}"),
            effects::this_gets::<1, 0>,
            "{R}: Vaevictis Asmadi gets +1/+0 until end of turn.",
        ),
        ability(
            cost("{G}"),
            effects::this_gets::<1, 0>,
            "{G}: Vaevictis Asmadi gets +1/+0 until end of turn.",
        ),
    ],
    ..oracle::VAEVICTIS_ASMADI
};
