use cantrip_api::*;
use cantrip_mechanics::{counter_unless_pays, targets, teamwork};
use cantrip_oracle as oracle;

/// Counter target spell unless its controller pays {2}. Counter that spell unless its controller pays {4} instead if
/// this spell was cast using teamwork.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ransom = if teamwork::used(item) { mana("{4}") } else { mana("{2}") };
    counter_unless_pays(g, item, 0, ransom)?;
    Ok(())
}

/// Teamwork 2 (as an additional cost to cast this spell, you may tap any number of creatures you control with total
/// power 2 or more): an optional cost announced before the target, which is chosen after it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effect),
    late_targets: true,
    optional_cost: Some(teamwork::optional(2)),
    labels: &["Cast without teamwork", "Teamwork 2: Tap any number of creatures you control with total power 2 or more"],
    ..oracle::WE_SAY_THEE_NAY
};
