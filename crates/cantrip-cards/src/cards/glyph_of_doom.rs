use cantrip_api::*;
use cantrip_mechanics::{delayed, history, targets};
use cantrip_oracle as oracle;

/// "Destroy all creatures that were blocked by that creature this turn" (CR 509.1h, 701.8a): the history of this turn's blocks,
/// so a creature that stopped being blocked, or a Wall that left, still counts.
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&wall) = g.event_objects(item).first() else { return Ok(()) };
    let blocked = history::creatures_blocked_by_this_turn(g, wall);
    g.destroy_all(&blocked);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::WALL_CREATURE], delayed::about_target_at_this_turns_next_end_of_combat),
    delayed: &[
        triggered(
            cantrip_mechanics::triggers::END_OF_COMBAT,
            delayed_1,
            "Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn.",
        ),
    ],
    ..oracle::GLYPH_OF_DOOM
};
