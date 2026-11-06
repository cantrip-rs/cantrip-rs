use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this creature enters, sacrifice it unless you sacrifice two Swamps." (CR 118.1).
fn paid_1(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Swamp))
}

/// "When this creature enters, sacrifice it unless you sacrifice two Swamps." (CR 608.2, 701.21a, 118.1).
fn sacrifice_unless_sacrifice_two_swamps(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let can_pay = g.permanents_of(item.controller).into_iter().filter(|&id| {
        paid_1(&*g, &Asker::new(item.object, item.controller), id)
            && g.can_sacrifice(item.controller, id)
    }).count() >= 2;
    if !(can_pay && g.may_pay(item.controller, item.object, item.object, sacrifice(2, paid_1), &PAY_QUESTION)?) {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "sacrifice two lands matching this ability?",
    "",
    "Sacrifice two lands",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_unless_sacrifice_two_swamps,
            "When this creature enters, sacrifice it unless you sacrifice two Swamps.",
        ),
    ],
    ..oracle::MOLD_DEMON
};
