use cantrip_api::*;
use cantrip_mechanics::{blight, damage_target, targets};
use cantrip_oracle as oracle;

/// "Cinder Strike deals 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional
/// cost was paid." A copy of the spell has the cost paid when the original did (CR 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, if blight::blighted(item) { 4 } else { 2 });
    Ok(())
}

/// "As an additional cost to cast this spell, you may blight 1" (CR 701.68a, 601.2b): an optional cost announced before the
/// target (CR 601.2b-c), which is chosen after it. Requiting Hex, Pyrrhic Strike and Burning Curiosity also blight as an
/// optional cost.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    late_targets: true,
    optional_cost: Some(blight::optional(1)),
    labels: &["Cast without blighting", "Blight 1: put a -1/-1 counter on a creature you control"],
    ..oracle::CINDER_STRIKE
};
