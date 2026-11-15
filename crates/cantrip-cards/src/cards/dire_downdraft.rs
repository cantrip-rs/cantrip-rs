use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if it targets an attacking or tapped creature." (CR 601.2f, 118.7a) and "Target
/// creature's owner puts it on their choice of the top or bottom of their library." (CR 401.1, 608.2d).
fn costs_less_if_it_targets_an_attacking_or_tapped_creature(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    let attacking = targets::costs_less_if_it_targets_an_attacking_creature::<1>(q, source, player, costed, targets);
    if attacking != CostDelta::NONE { attacking } else { targets::costs_less_if_it_targets_a_tapped_creature::<1>(q, source, player, costed, targets) }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], library::owner_puts_first_target_on_top_or_bottom),
    own_cost_changes: &[costs_less_if_it_targets_an_attacking_or_tapped_creature],
    ..oracle::DIRE_DOWNDRAFT
};
