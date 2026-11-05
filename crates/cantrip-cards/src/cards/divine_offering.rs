use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets};
use cantrip_oracle as oracle;

/// Destroy target artifact. You gain life equal to its mana value (CR 608.2h, 701.8a).
fn offering(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = destroy_target(g, item, 0) else { return Ok(()) };
    g.end_sentence();
    let value = i64::from(g.mana_value(artifact));
    g.gain_life(item.controller, value);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT], offering),
    ..oracle::DIVINE_OFFERING
};
