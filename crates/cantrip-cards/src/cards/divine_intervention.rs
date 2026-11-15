use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "When you remove the last intervention counter from this enchantment" (CR 603.2): the counters coming off leave it with
/// none, and the player who removed them is its controller - by its own upkeep ability, by a cost they paid or by an effect
/// they control, however that happens; one another player removes doesn't trigger it (its rulings). The event is recorded
/// only when at least one counter came off.
fn you_remove_the_last_intervention_counter(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::CountersRemoved { object, kind: CounterKind::Intervention, total: 0, by: Some(by), .. }
            if object == asker.source && by == asker.controller =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "... the game is a draw." (CR 104.4c): nobody wins and nobody loses.
fn the_game_is_a_draw(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    g.draw_game();
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // The ability's controller removes the counter (CR 608.2c), so it is "you" for the next ability only while they still
        // control this enchantment.
        triggered(
            triggers::YOUR_UPKEEP,
            effects::remove_a_counter_from_this::<{ CounterKind::Intervention as usize }>,
            "At the beginning of your upkeep, remove an intervention counter from this enchantment.",
        ),
        triggered(
            when_asked(EventMask::COUNTERS, you_remove_the_last_intervention_counter),
            the_game_is_a_draw,
            "When you remove the last intervention counter from this enchantment, the game is a draw.",
        ),
    ],
    // "This enchantment enters with two intervention counters on it." (CR 614.1c, 122.6).
    enters: Some(effects::enters_with_counters::<{ CounterKind::Intervention as usize }, 2>),
    ..oracle::DIVINE_INTERVENTION
};
