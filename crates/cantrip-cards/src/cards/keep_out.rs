use cantrip_api::*;
use cantrip_mechanics::{damage_target, destroy_target, targets};
use cantrip_oracle as oracle;

/// "Keep Out deals 4 damage to target tapped creature."
fn damage_tapped_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

/// "Destroy target enchantment."
fn destroy_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(damage_tapped_creature, "Keep Out deals 4 damage to target tapped creature.").targets(&[targets::TAPPED_CREATURE]),
    mode(destroy_enchantment, "Destroy target enchantment.").targets(&[targets::ENCHANTMENT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::KEEP_OUT };
