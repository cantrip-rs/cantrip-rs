use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, teamwork};
use cantrip_oracle as oracle;

/// "Helicarrier Strike deals 2 damage to target attacking or blocking creature. If this spell was cast using teamwork, it deals 4
/// damage to that creature instead." (CR 702.194b, 120.3, 608.2b): the 4 replaces the 2, so only one of them is dealt; a copy of
/// a spell cast using teamwork was cast that way too (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, if teamwork::used(item) { 4 } else { 2 });
    Ok(())
}

/// Teamwork 2 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 2 or
/// more): an optional cost announced before the target, which is chosen after it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_OR_BLOCKING_CREATURE], effect),
    late_targets: true,
    optional_cost: Some(teamwork::optional(2)),
    labels: &["Cast without teamwork", "Teamwork 2: Tap any number of creatures you control with total power 2 or more"],
    ..oracle::HELICARRIER_STRIKE
};
