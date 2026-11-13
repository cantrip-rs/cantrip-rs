use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your draw step, this artifact deals damage equal to the number of doom counters on it to each player." (CR 608.2, 120.3).
fn artifact_deals_damage_equal_number(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_1 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .map(|o: ObjectId| i64::from(q.state().obj(o).counters(CounterKind::Doom)))
            .sum::<i64>() as u32
    };
    let recipients_1 = {
        let q: &dyn Query = g;
        q.state()
            .apnap()
            .collect::<Vec<PlayerId>>()
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, amount_1))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "{4}: Remove a doom counter from this artifact. Any player may activate this ability but only during any upkeep step." (CR 608.2).
fn remove_doom_counter_artifact_any(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_2_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_2_0 {
                let _ = g.remove_counters(object, CounterKind::Doom, 1);
            }
        },
    );
    Ok(())
}

/// "{4}: Remove a doom counter from this artifact. Any player may activate this ability but only during any upkeep step." (CR 602.5).
fn remove_doom_counter_artifact_any_if(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::Upkeep
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            effects::counters_on_self::<{ CounterKind::Doom as usize }, 1>,
            "At the beginning of your upkeep, put a doom counter on this artifact.",
        ),
        triggered(
            triggers::YOUR_DRAW_STEP,
            artifact_deals_damage_equal_number,
            "At the beginning of your draw step, this artifact deals damage equal to the number of doom counters on it to each player.",
        ),
        ability(
                cost("{4}"),
                remove_doom_counter_artifact_any,
                "{4}: Remove a doom counter from this artifact. Any player may activate this ability but only during any upkeep step.",
            )
            .condition(remove_doom_counter_artifact_any_if)
            .any_player_may_activate(),
    ],
    ..oracle::ARMAGEDDON_CLOCK
};
