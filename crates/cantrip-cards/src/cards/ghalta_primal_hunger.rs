use cantrip_api::*;
use cantrip_mechanics::total_power_of_creatures_you_control;
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the total power of creatures you control" (CR 601.2f, 118.7a): the
/// creatures its caster controls as the total cost is determined, a total below zero counting as nothing (CR 107.1b), and
/// only the generic part of the cost reduced. Only casting the spell is reduced, never a special action taken for the
/// card (CR 116.2).
fn less_by_total_power(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(total_power_of_creatures_you_control(q, player).max(0) as u32)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_by_total_power],
    ..oracle::GHALTA_PRIMAL_HUNGER
};
