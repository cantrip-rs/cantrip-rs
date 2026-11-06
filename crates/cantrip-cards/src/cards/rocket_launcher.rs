use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "{2}: This artifact deals 1 damage to any target. Destroy this artifact at the beginning of the next end step. Activate only if you've controlled this artifact continuously since the beginning of your most recent turn." (CR 608.2, 701.8a).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_3 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    g.destroy_all(&objects_3);
    Ok(())
}

/// "{2}: This artifact deals 1 damage to any target. Destroy this artifact at the beginning of the next end step. Activate only if you've controlled this artifact continuously since the beginning of your most recent turn." (CR 608.2, 120.3).
fn artifact_deals_1_damage_any(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.damage_together(&[(item.object, g.state().target(item, 0), 1)]);
    g.end_sentence();
    delayed::at_next_end_step(g, item, 0, Target::None);
    Ok(())
}

/// "{2}: This artifact deals 1 damage to any target. Destroy this artifact at the beginning of the next end step. Activate only if you've controlled this artifact continuously since the beginning of your most recent turn." (CR 608.2).
fn artifact_deals_1_damage_any_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            q.state().obj(candidate).controlled_since
                < q.state().player(q.controller(candidate)).turn_began
        })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}"),
                artifact_deals_1_damage_any,
                "{2}: This artifact deals 1 damage to any target. Destroy this artifact at the beginning of the next end step. Activate only if you've controlled this artifact continuously since the beginning of your most recent turn.",
            )
            .targets(&[targets::ANY])
            .condition(artifact_deals_1_damage_any_if),
    ],
    delayed: &[
        delayed_trigger(
            delayed_2,
            "Destroy this artifact at the beginning of the next end step.",
        ),
    ],
    ..oracle::ROCKET_LAUNCHER
};
