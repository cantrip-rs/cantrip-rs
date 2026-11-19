use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast for each creature that attacked this turn" (CR 601.2f, 118.7a, 508.1): the creatures declared
/// as attackers this turn, whoever controls them now (only the active player declares attackers, CR 506.2). A creature that attacked
/// in two combats counts once, one put onto the battlefield attacking was never declared (CR 508.4), and one that has since left the
/// battlefield still attacked. Counted as the cost is determined, and never off a colored symbol (CR 118.7a, which the core leaves in place).
fn less_for_each_creature_that_attacked(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(history::creatures_attacked_with(q, q.state().turn.active) as u32)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 3)),
    own_cost_changes: &[less_for_each_creature_that_attacked],
    ..oracle::ROWDY_RESEARCH
};
