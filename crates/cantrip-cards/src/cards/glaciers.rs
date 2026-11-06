use cantrip_api::*;
use cantrip_mechanics::{lands, statics, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{U}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_enchantment_unless_pay_w(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{W}{U}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{U}. All Mountains are Plains." (CR 604.1).
fn sacrifice_enchantment_unless_pay_w_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Mountain))
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
            sacrifice_enchantment_unless_pay_w,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{U}.",
        ),
    ],
    statics: &[
        sets_land_subtypes(
            sacrifice_enchantment_unless_pay_w_affects,
            statics::set_land_type::<{ lands::sub(Subtype::Plains) }>,
            "All Mountains are Plains.",
        ),
    ],
    ..oracle::GLACIERS
};
