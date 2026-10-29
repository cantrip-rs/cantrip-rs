use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Tap target creature an opponent controls. Put a stun counter on it." (CR 701.26a, 122.1d): both are done to the target
/// as it resolves; nothing if it is no longer legal (CR 608.2b). The counter goes on whether or not the tap did anything.
fn freeze(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.end_sentence();
    g.add_counters(creature, CounterKind::Stun, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        freeze,
        "When this creature enters, tap target creature an opponent controls. Put a stun counter on it.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::FRACTAL_MASCOT
};
