use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if creatures you control have total toughness 10 or greater" (CR 601.2f, 118.7a):
/// asked as the cost is determined, of the creatures the caster controls then. Orysa is on the stack, so she is not counted.
/// A creature with negative toughness counts as negative (CR 107.1b), and the generic mana is all it takes off.
fn less_for_total_toughness_ten(q: &dyn Query, _: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let total: i32 = q
        .permanents_of(player)
        .into_iter()
        .filter(|&id| q.chars(id).is(CardType::Creature))
        .map(|id| i32::from(q.chars(id).toughness.unwrap_or(0)))
        .sum();
    if matches!(costed, Costed::Spell(_)) && total >= 10 { CostDelta::less(3) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_for_total_toughness_ten],
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 2), "When Orysa enters, draw two cards.")],
    ..oracle::ORYSA_TIDE_CHOREOGRAPHER
};
