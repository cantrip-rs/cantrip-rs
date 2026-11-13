use cantrip_api::*;
use cantrip_mechanics::{fight, is_mount_or_vehicle, targets};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if it targets a Mount or Vehicle you control" (CR 601.2c, 601.2f, 118.7a): a target,
/// once it is announced, that is a Mount or a Vehicle on the battlefield under the caster's control. "Mount" and "Vehicle"
/// are the words of this spell's own text (CR 612).
fn costs_less_if_it_targets_a_mount_or_vehicle_you_control(q: &dyn Query, source: ObjectId, caster: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let it = |object: ObjectId| targets::permanent(q, object) && q.controller(object) == caster && is_mount_or_vehicle(q, source, object);
    if targets.iter().filter_map(|t| t.object()).any(it) {
        CostDelta::less(1)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], fight::first_target_bites_second),
    own_cost_changes: &[costs_less_if_it_targets_a_mount_or_vehicle_you_control],
    ..oracle::RUN_OVER
};
