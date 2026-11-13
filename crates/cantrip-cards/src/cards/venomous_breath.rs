use cantrip_api::*;
use cantrip_mechanics::{delayed, history, targets, triggers};
use cantrip_oracle as oracle;

/// "Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn." (CR 608.2, 701.8a, 509.1).
/// Both are asked of this turn's blocks (CR 509.1h): those it blocked, and those that blocked it, each still a creature on the
/// battlefield, whether or not it still is.
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&it) = g.event_objects(item).first() else { return Ok(()) };
    let mut doomed = history::creatures_blocked_by_this_turn(g, it);
    for record in g.history().select(EventQuery::kind(EventMask::BLOCKS).object(it).this_turn()) {
        if let Event::Blocks { blocker, attacker, .. } = record.event {
            if attacker == it && !doomed.contains(&blocker) && g.state().battlefield().contains(&blocker) && g.chars(blocker).is(CardType::Creature) {
                doomed.push(blocker);
            }
        }
    }
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], delayed::about_target_at_this_turns_next_end_of_combat),
    delayed: &[
        triggered(
            triggers::END_OF_COMBAT,
            delayed_1,
            "Choose target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn.",
        ),
    ],
    ..oracle::VENOMOUS_BREATH
};
