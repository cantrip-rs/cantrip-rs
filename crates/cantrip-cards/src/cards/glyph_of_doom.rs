use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Destroy all creatures that were blocked by that creature this turn" (CR 509.1h, 701.8a): the history of this turn's blocks,
/// so a creature that stopped being blocked, or a Wall that left, still counts.
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&wall) = g.event_objects(item).first() else { return Ok(()) };
    let mut blocked: Vec<ObjectId> = Vec::new();
    for record in g.history().this_turn().iter() {
        if let Event::Blocks { blocker, attacker, .. } = record.event {
            if blocker == wall && !blocked.contains(&attacker) {
                blocked.push(attacker);
            }
        }
    }
    blocked.retain(|&id| g.state().battlefield().contains(&id) && g.chars(id).is(CardType::Creature));
    g.destroy_all(&blocked);
    Ok(())
}

/// "Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn." (CR 608.2).
fn choose_target_wall_creature_turns(
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

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::WALL_CREATURE],
        choose_target_wall_creature_turns,
    ),
    delayed: &[
        triggered(
            cantrip_mechanics::triggers::END_OF_COMBAT,
            delayed_1,
            "Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn.",
        ),
    ],
    ..oracle::GLYPH_OF_DOOM
};
