use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}{U}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_enchantment_unless_pay_u(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{U}{U}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "Blue creatures get +1/+1." (CR 604.1).
fn blue_creatures_get_1_1_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).colors.has(q.words(asker.source).color(Color::Blue))
        && (*so_far).is(CardType::Creature)
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
            sacrifice_enchantment_unless_pay_u,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}{U}.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            blue_creatures_get_1_1_affects,
            cantrip_mechanics::statics::plus_pt_saturating::<1, 1>,
            "Blue creatures get +1/+1.",
        ),
    ],
    ..oracle::SUNKEN_CITY
};
