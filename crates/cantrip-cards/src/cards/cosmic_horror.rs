use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, destroy this creature unless you pay {3}{B}{B}{B}. If this creature is destroyed this way, it deals 7 damage to you." (CR 608.2, 120.3, 701.8a).
fn destroy_creature_unless_pay_3(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    let mut hoisted_3: Vec<ObjectId> = Vec::new();
    let paid_0 = g
        .may_pay(
            item.controller,
            item.object,
            item.object,
            cost("{3}{B}{B}{B}"),
            &PAY_QUESTION,
        )?;
    g.end_sentence();
    if !(((paid_0 as i64) != 0)) {
        let objects_1 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
        let destroyed_1 = g.destroy_all(&objects_1);
        g.end_sentence();
        if (destroyed_1.clone().len() as i64) > 0 {
            let recipients_2 = Some(item.controller)
                .into_iter()
                .map(Target::Player)
                .collect::<Vec<Target>>();
            let batch: Vec<(ObjectId, Target, u32)> = recipients_2
                .iter()
                .map(|t| (item.object, *t, 7))
                .collect();
            g.damage_together(&batch);
            g.end_sentence();
        }
        g.end_sentence();
        hoisted_3.extend(destroyed_1.clone());
        hoisted_4
            .extend({
                let q: &dyn Query = g;
                objects_1
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
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
            destroy_creature_unless_pay_3,
            "At the beginning of your upkeep, destroy this creature unless you pay {3}{B}{B}{B}. If this creature is destroyed this way, it deals 7 damage to you.",
        ),
    ],
    ..oracle::COSMIC_HORROR
};
