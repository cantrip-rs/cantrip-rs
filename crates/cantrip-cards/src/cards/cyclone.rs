use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a wind counter on this enchantment, then sacrifice this enchantment unless you pay {G} for each wind counter on it. If you pay, this enchantment deals damage equal to the number of wind counters on it to each creature and each player." (CR 608.2, 120.3, 701.21a).
fn put_wind_counter_enchantment_sacrifice(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_0: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Wind, 1));
        }
    }
    g.add_counters_together(&counters_0);
    let paid_1 = g
        .may_pay(
            item.controller,
            item.object,
            item.object,
            Cost::mana(
                mana("{G}")
                    .times(
                        ({
                            let q: &dyn Query = g;
                            Some(item.object)
                                .into_iter()
                                .map(|o: ObjectId| i64::from(
                                    q.state().obj(o).counters(CounterKind::Wind),
                                ))
                                .sum::<i64>()
                        })
                            .max(0) as u32,
                    )
                    .unwrap_or(ManaCost {
                        generic: 255,
                        ..ManaCost::ZERO
                    }),
            ),
            &PAY_QUESTION,
        )?;
    g.end_sentence();
    if (paid_1 as i64) != 0 {
        let amount_2 = {
            let q: &dyn Query = g;
            Some(item.object)
                .into_iter()
                .map(|o: ObjectId| i64::from(
                    q.state().obj(o).counters(CounterKind::Wind),
                ))
                .sum::<i64>() as u32
        };
        let recipients_2 = {
            let q: &dyn Query = g;
            {
                let mut all: Vec<Target> = Vec::new();
                for t in q
                    .state()
                    .battlefield()
                    .iter()
                    .copied()
                    .filter(|&id| q.chars(id).is(CardType::Creature))
                    .collect::<Vec<ObjectId>>()
                    .into_iter()
                    .map(Target::Object)
                    .collect::<Vec<Target>>()
                {
                    if !all.contains(&t) {
                        all.push(t);
                    }
                }
                for t in q
                    .state()
                    .apnap()
                    .collect::<Vec<PlayerId>>()
                    .into_iter()
                    .map(Target::Player)
                    .collect::<Vec<Target>>()
                {
                    if !all.contains(&t) {
                        all.push(t);
                    }
                }
                all
            }
        };
        let batch: Vec<(ObjectId, Target, u32)> = recipients_2
            .iter()
            .map(|t| (item.object, *t, amount_2 as u32))
            .collect();
        g.damage_together(&batch);
        g.end_sentence();
    } else {
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
            put_wind_counter_enchantment_sacrifice,
            "At the beginning of your upkeep, put a wind counter on this enchantment, then sacrifice this enchantment unless you pay {G} for each wind counter on it. If you pay, this enchantment deals damage equal to the number of wind counters on it to each creature and each player.",
        ),
    ],
    ..oracle::CYCLONE
};
