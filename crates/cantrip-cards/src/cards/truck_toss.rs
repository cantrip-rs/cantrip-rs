use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if you control a Vehicle" (CR 601.2f, 301.7): judged as the total cost is determined, once,
/// and generic mana only (CR 118.7a). A permanent with the Vehicle subtype counts whether or not it is a creature right now.
fn less_with_a_vehicle(q: &dyn Query, _: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let vehicle = q.permanents_of(player).into_iter().any(|id| q.chars(id).has_subtype(Subtype::Vehicle));
    if matches!(costed, Costed::Spell(_)) && vehicle { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Truck Toss deals 4 damage to any target."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ANY], effect), own_cost_changes: &[less_with_a_vehicle], ..oracle::TRUCK_TOSS };
