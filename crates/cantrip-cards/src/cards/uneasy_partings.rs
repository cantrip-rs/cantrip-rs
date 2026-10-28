use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if it targets an attacking nontoken creature" (CR 601.2c, 601.2f): the target as the total
/// cost is determined, once it is announced; generic mana only (CR 118.7a). A token (CR 111.1) is not a nontoken creature.
fn less_for_an_attacking_nontoken_creature(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let attacking = targets.iter().filter_map(|t| t.object()).any(|creature| q.is_attacking(creature) && !q.obj(creature).token);
    if attacking { CostDelta::less(1) } else { CostDelta::NONE }
}

/// "Target creature's owner puts it on their choice of the top or bottom of their library." The owner (CR 108.3), not the
/// controller, chooses, and puts all of what is put there in the order they wish. With its target gone the spell does not
/// resolve (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    library::owner_puts_on_top_or_bottom(g, item.object, creature)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    own_cost_changes: &[less_for_an_attacking_nontoken_creature],
    ..oracle::UNEASY_PARTINGS
};
