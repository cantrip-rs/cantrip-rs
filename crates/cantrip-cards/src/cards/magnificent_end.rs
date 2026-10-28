use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Magnificent End deals 5 damage to target creature."
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], damage),
    own_cost_changes: &[targets::costs_less_if_it_targets_a_tapped_creature::<3>],
    ..oracle::MAGNIFICENT_END
};
