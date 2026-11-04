use cantrip_api::*;
use cantrip_mechanics::{choose, history, triggers};
use cantrip_oracle as oracle;

/// "This creature enters with seven +1/+0 counters on it. At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it. {X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than seven. Activate only during your upkeep." (CR 614.1c, 122.6).
fn put_up_x_1_0_enters(
    _: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.counters[CounterKind::Plus1Plus0 as usize] = entering
        .counters[CounterKind::Plus1Plus0 as usize]
        .saturating_add(7);
    Ok(())
}

/// "At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it." (CR 603.4).
fn if_creature_attacked_blocked_combat_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            ({
                let history_candidate = candidate;
                {
                    let mut attackers: Vec<ObjectId> = history::this_combat(q)
                        .iter()
                        .filter_map(|r: &Record| -> Option<Fired> {
                            let event = &r.event;
                            if let Event::Attacks { attacker, .. } = event {
                                let id = *attacker;
                                let subject = Target::Object(id);
                                if history_candidate == id {
                                    return Fired::about(subject);
                                }
                            }
                            None
                        })
                        .filter_map(|f: Fired| f.subject.object())
                        .collect();
                    attackers.sort();
                    attackers.dedup();
                    attackers.len() as i64
                }
            }) >= 1
                || ({
                    let history_candidate = candidate;
                    history::this_combat(q)
                        .iter()
                        .filter_map(|r: &Record| -> Option<Fired> {
                            let event = &r.event;
                            if let Event::Blocks { blocker, declared: true, .. } = event {
                                let id = *blocker;
                                let subject = Target::Object(id);
                                if history_candidate == id {
                                    return Fired::about(subject);
                                }
                            }
                            None
                        })
                        .count() as i64
                }) >= 1
        })
}

/// "At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it." (CR 608.2).
fn if_creature_attacked_blocked_combat(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_1_0 {
                let _ = g.remove_counters(object, CounterKind::Plus1Plus0, 1);
            }
        },
    );
    Ok(())
}

/// "{X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than seven. Activate only during your upkeep." (CR 608.2, 122.1).
fn put_up_x_1_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !is_live(g, item.object) { return Ok(()) }
    let chosen = choose::number(g, item.controller, item.object, 0, u32::from(item.x), "Put counters")?;
    g.add_counters_with_limit(item.object, CounterKind::Plus1Plus0, chosen as i32, 7);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::END_OF_COMBAT,
                if_creature_attacked_blocked_combat,
                "At end of combat, if this creature attacked or blocked this combat, remove a +1/+0 counter from it.",
            )
            .condition(if_creature_attacked_blocked_combat_if),
        ability(
                cost("{X}").and(TAP),
                put_up_x_1_0,
                "{X}, {T}: Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than seven. Activate only during your upkeep.",
            )
            .condition(cantrip_mechanics::your_upkeep),
    ],
    enters: Some(put_up_x_1_0_enters),
    ..oracle::CLOCKWORK_BEAST
};
