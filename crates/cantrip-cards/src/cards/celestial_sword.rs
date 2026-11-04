use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Its controller sacrifices it at the beginning of the next end step." (CR 608.2, 701.21a).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for object in about(g, item, 0) {
        g.sacrifice(g.controller(object), &[object]);
    }
    Ok(())
}

/// "{3}, {T}: Target creature you control gets +3/+3 until end of turn. Its controller sacrifices it at the beginning of the next end step." (CR 608.2, 611.2a).
fn target_creature_control_gets_3(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = targets::object(g, item, 0);
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                cantrip_mechanics::pump(g, item.object, object, 3, 3);
            }
        },
    );
    g.end_sentence();
    {
        let about: Vec<ObjectId> = t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .collect::<Vec<ObjectId>>();
        {
            g.delayed_trigger_about(
                DelayedWhen::Step {
                    step: Step::End,
                    on_turn_of: None,
                },
                item.object,
                item.controller,
                AbilityRef::delayed(item.ability.card, 0),
                &about,
            );
        }
    }
    Ok(())
}

/// "Its controller sacrifices it at the beginning of the next end step." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    q.event_objects(item)
        .split(|&id| id == ObjectId::NONE)
        .nth(part)
        .map_or_else(Vec::new, <[ObjectId]>::to_vec)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{3}").and(TAP),
                target_creature_control_gets_3,
                "{3}, {T}: Target creature you control gets +3/+3 until end of turn. Its controller sacrifices it at the beginning of the next end step.",
            )
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    delayed: &[
        delayed_trigger(
            delayed_2,
            "Its controller sacrifices it at the beginning of the next end step.",
        ),
    ],
    ..oracle::CELESTIAL_SWORD
};
