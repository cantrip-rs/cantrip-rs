use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if you've committed a crime this turn" (CR 601.2f, 700.13): asked as the total cost is
/// determined, of the crimes committed earlier this turn.
fn less_after_a_crime(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if history::committed_a_crime_this_turn(q.history(), player) {
        CostDelta::less(1)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 2)),
    own_cost_changes: &[less_after_a_crime],
    ..oracle::SEIZE_THE_SECRETS
};
