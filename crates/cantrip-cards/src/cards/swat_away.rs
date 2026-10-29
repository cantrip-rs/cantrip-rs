use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if a creature is attacking you" (CR 601.2f, 700.4): judged once, as the total cost is
/// determined, for the player casting it. A creature attacking a planeswalker or a battle of theirs is not attacking them (CR
/// 506.3a), and an attacker of their own attacking someone else is not either; generic mana only (CR 118.7a).
fn less_if_a_creature_attacks_you(q: &dyn Query, _: ObjectId, caster: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if q.state().attackers.iter().any(|a| a.target == Target::Player(caster)) { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "The owner of target spell or creature puts it on their choice of the top or bottom of their library." The owner (CR 108.3),
/// not the controller, chooses; a spell leaves the stack without being countered. With its target gone the spell does not
/// resolve (CR 608.2b).
fn put_on_top_or_bottom(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    library::owner_puts_on_top_or_bottom(g, item.object, object)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_if_a_creature_attacks_you],
    spell: spell(&[targets::SPELL_OR_CREATURE], put_on_top_or_bottom),
    ..oracle::SWAT_AWAY
};
