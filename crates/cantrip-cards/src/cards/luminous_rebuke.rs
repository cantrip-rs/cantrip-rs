use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if it targets a tapped creature. Destroy target creature." The reduction is applied once the
/// target is chosen (CR 601.2c, 601.2f) and is shared with Ajani's Response, which prints the same text.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::destroy_first_target),
    own_cost_changes: &[targets::costs_less_if_it_targets_a_tapped_creature::<3>],
    ..oracle::LUMINOUS_REBUKE
};
