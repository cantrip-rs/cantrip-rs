use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature."
fn destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], destroy),
    own_cost_changes: &[targets::costs_less_if_it_targets_a_tapped_creature::<3>],
    ..oracle::AJANIS_RESPONSE
};
