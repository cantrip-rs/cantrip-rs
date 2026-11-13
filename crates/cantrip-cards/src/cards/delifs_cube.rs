use cantrip_api::*;
use cantrip_mechanics::{delayed, regenerate, targets, triggers};
use cantrip_oracle as oracle;

/// "This turn, when target creature you control attacks and isn't blocked, it assigns no combat damage this turn and you put a cube counter on this artifact." (CR 608.2, 611.2a, 122.1).
fn watch_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_3 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for &object in &objects_3 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::NoCombatDamageAssignment,
                    Duration::EndOfTurn,
                );
            }
        },
    );
    let mut counters_4: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_4.push((object, CounterKind::Cube, 1i32));
        }
    }
    g.add_counters_together(&counters_4);
    Ok(())
}

/// "{2}, {T}: This turn, when target creature you control attacks and isn't blocked, it assigns no combat damage this turn and you put a cube counter on this artifact." (CR 608.2).
fn when_target_creature_control_attacks(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let Some(watched) = t_0.get(0).copied().and_then(|t: Target| t.object()) else { return Ok(()) };
    delayed::this_turn_watching(g, item, watched);
    Ok(())
}

/// "{2}, Remove a cube counter from this artifact: Regenerate target creature." (CR 608.2).
fn regenerate_target_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                when_target_creature_control_attacks,
                "{2}, {T}: This turn, when target creature you control attacks and isn't blocked, it assigns no combat damage this turn and you put a cube counter on this artifact.",
            )
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        ability(
                cost("{2}").and(remove_counters(CounterKind::Cube, 1)),
                regenerate_target_creature,
                "{2}, Remove a cube counter from this artifact: Regenerate target creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[
        triggered(
            when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
            watch_2,
            "This turn, when target creature you control attacks and isn't blocked, it assigns no combat damage this turn and you put a cube counter on this artifact.",
        ),
    ],
    ..oracle::DELIFS_CUBE
};
