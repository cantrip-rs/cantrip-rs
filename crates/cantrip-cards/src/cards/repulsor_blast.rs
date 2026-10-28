use cantrip_api::*;
use cantrip_mechanics::{targets, teamwork};
use cantrip_oracle as oracle;

/// "Repulsor Blast deals 5 damage to target creature. If this spell was cast using teamwork, it also deals 2 damage to that
/// creature's controller." (CR 702.194b, 120.3, 608.2b): the controller is the creature's as the spell resolves, before the damage
/// can kill it; a spell whose target has become illegal does nothing. A copy of a spell cast using teamwork was cast that way
/// too (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(creature);
    g.damage(item.object, Target::Object(creature), 5);
    if teamwork::used(item) {
        g.damage(item.object, Target::Player(controller), 2);
    }
    Ok(())
}

/// Teamwork 2 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 2 or
/// more): an optional cost announced before the target, which is chosen after it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    late_targets: true,
    optional_cost: Some(teamwork::optional(2)),
    labels: &["Cast without teamwork", "Teamwork 2: Tap any number of creatures you control with total power 2 or more"],
    ..oracle::REPULSOR_BLAST
};
