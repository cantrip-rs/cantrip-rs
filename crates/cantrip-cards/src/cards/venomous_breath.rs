use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn." (CR 608.2, 701.8a).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && (({
                        let history_candidate = id;
                        q
                            .history()
                            .this_turn()
                            .iter()
                            .filter_map(|r: &Record| -> Option<Fired> {
                                let event = &r.event;
                                if let Event::Blocks { blocker, attacker, .. } = event {
                                    let id = *blocker;
                                    let other = *attacker;
                                    let subject = Target::Object(id);
                                    if history_candidate == id
                                        && about(q, item, 0).first().copied() == Some(other)
                                    {
                                        return Fired::about(subject);
                                    }
                                }
                                None
                            })
                            .count() as i64
                    }) >= 1
                        || ({
                            let history_candidate = id;
                            q
                                .history()
                                .this_turn()
                                .iter()
                                .filter_map(|r: &Record| -> Option<Fired> {
                                    let event = &r.event;
                                    if let Event::Blocks { blocker, attacker, .. } = event {
                                        let id = *attacker;
                                        let other = *blocker;
                                        let subject = Target::Object(id);
                                        if history_candidate == id
                                            && about(q, item, 0).first().copied() == Some(other)
                                        {
                                            return Fired::about(subject);
                                        }
                                    }
                                    None
                                })
                                .count() as i64
                        }) >= 1)
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_2);
    Ok(())
}

/// "Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn." (CR 608.2).
fn choose_target_creature_turns_next(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let about: Vec<ObjectId> = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.delayed_trigger_about(
        DelayedWhen::NextThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &about,
    );
    Ok(())
}

/// "Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    assert_eq!(part, 0, "one real-object list");
    q.event_objects(item).to_vec()
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], choose_target_creature_turns_next),
    delayed: &[
        triggered(
            triggers::END_OF_COMBAT,
            delayed_1,
            "Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn.",
        ),
    ],
    ..oracle::VENOMOUS_BREATH
};
