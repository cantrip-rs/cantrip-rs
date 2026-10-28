use cantrip_api::*;
use cantrip_mechanics::history::creatures_died_this_turn;
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if a creature died this turn" (CR 601.2f, 700.4): judged once, as the total cost is
/// determined, from the event history (any creature's, by last known information), and generic mana only (CR 118.7a).
fn less_if_a_creature_died(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if creatures_died_this_turn(q, None) > 0 { CostDelta::less(3) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef { own_cost_changes: &[less_if_a_creature_died], ..oracle::DREADED_BAT_CLOUD };
