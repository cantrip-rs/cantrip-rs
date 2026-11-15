use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if it targets a blocking creature" (CR 601.2c, 601.2f): the target as the total cost is
/// determined, once it is announced; generic mana only (CR 118.7a). Only a creature can be blocking (CR 506.4, 509.1).
fn costs_one_less_if_it_targets_a_blocking_creature(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let blocking = targets.iter().filter_map(|t| t.object()).any(|creature| q.state().blocks.iter().any(|b| b.blocker == creature));
    if blocking { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_indestructible::<2, 0>),
    own_cost_changes: &[costs_one_less_if_it_targets_a_blocking_creature],
    ..oracle::DARK_ENDURANCE
};
