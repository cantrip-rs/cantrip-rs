use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, this creature deals 8 damage to you unless you pay {G}{G}{G}{G}." (CR 608.2, 120.3, 118.1).
fn creature_deals_8_damage_unless(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let paid_0 = g
        .may_pay(
            item.controller,
            item.object,
            item.object,
            cost("{G}{G}{G}{G}"),
            &PAY_QUESTION,
        )?;
    g.end_sentence();
    if !(((paid_0 as i64) != 0)) {
        let recipients_1 = Some(item.controller)
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>();
        let batch: Vec<(ObjectId, Target, u16)> = recipients_1
            .iter()
            .map(|t| (item.object, *t, 8u16))
            .collect();
        g.damage_together(&batch);
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
        triggered(
            triggers::YOUR_UPKEEP,
            creature_deals_8_damage_unless,
            "At the beginning of your upkeep, this creature deals 8 damage to you unless you pay {G}{G}{G}{G}.",
        ),
    ],
    ..oracle::FORCE_OF_NATURE
};
