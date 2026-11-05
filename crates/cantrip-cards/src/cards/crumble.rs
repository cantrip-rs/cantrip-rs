use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets};
use cantrip_oracle as oracle;

/// Destroy the artifact without regeneration, then give its controller its mana value in life (CR 701.19c, 608.2h).
fn crumble(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(artifact) = destroy_target_no_regen(g, item, 0) {
        g.end_sentence();
        let amount = i64::from(g.mana_value(artifact));
        let controller = g.controller(artifact);
        g.gain_life(controller, amount);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT], crumble),
    ..oracle::CRUMBLE
};
