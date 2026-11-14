use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if it targets a tapped creature." (CR 601.2f: the cost is fixed once the target is
/// chosen.) "Destroy target creature." (CR 701.8a, 608.2b: nothing if the target is no longer legal.)
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::destroy_first_target),
    own_cost_changes: &[targets::costs_less_if_it_targets_a_tapped_creature::<3>],
    ..oracle::SEIZED_FROM_SLUMBER
};
