use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_enchantment_unless_pay_w(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{W}{W}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}. All Mountains are Plains." (CR 604.1).
fn sacrifice_enchantment_unless_pay_w_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Mountain))
}

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}. All Mountains are Plains." (CR 604.1).
fn sacrifice_enchantment_unless_pay_w_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    chars
        .set_land_subtypes(
            SubtypeSet::NONE.with(q.words(asker.source).subtype(Subtype::Plains)),
        );
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Don't pay",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_enchantment_unless_pay_w,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.",
        ),
    ],
    statics: &[
        sets_land_subtypes(
            sacrifice_enchantment_unless_pay_w_affects,
            sacrifice_enchantment_unless_pay_w_apply_0,
            "All Mountains are Plains.",
        ),
    ],
    ..oracle::CONVERSION
};
