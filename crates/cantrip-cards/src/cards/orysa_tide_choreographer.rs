use cantrip_api::*;
use cantrip_mechanics::{effects, total_toughness_of_creatures_you_control, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if creatures you control have total toughness 10 or greater" (CR 601.2f, 118.7a):
/// asked as the cost is determined, of the creatures the caster controls then. Orysa is on the stack, so she is not counted.
/// A creature with negative toughness counts as negative (CR 107.1b), and the generic mana is all it takes off.
fn less_for_total_toughness_ten(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let total = total_toughness_of_creatures_you_control(q, player);
    if total >= 10 { CostDelta::less(3) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_for_total_toughness_ten],
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 2), "When Orysa enters, draw two cards.")],
    ..oracle::ORYSA_TIDE_CHOREOGRAPHER
};
