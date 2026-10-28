use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if it targets an attacking creature" (CR 601.2c, 601.2f): the target as the total cost
/// is determined, once it is announced; generic mana only (CR 118.7a). What happens to the creature afterwards changes
/// nothing paid.
fn less_for_an_attacking_target(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let attacking = targets.iter().filter_map(|t| t.object()).any(|creature| q.is_attacking(creature));
    if attacking { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Target creature gets -4/-0 until end of turn. Draw a card." A target no longer legal means the spell does not resolve
/// at all and draws nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, -4, 0);
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    own_cost_changes: &[less_for_an_attacking_target],
    ..oracle::DEPOWER
};
