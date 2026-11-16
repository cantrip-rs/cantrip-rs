use cantrip_api::*;
use cantrip_mechanics::cost_reduction;
use cantrip_oracle as oracle;

/// "During turns other than yours, spells you cast cost {1} less to cast." (CR 601.2f, 118.7, 604.1): its controller's spells
/// while it is not their turn, a generic reduction that takes only the generic part of the cost (CR 118.7a).
fn spells_you_cast_cost_1_less_during_turns_other_than_yours(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    if q.state().turn.active == q.controller(source) {
        return CostDelta::NONE;
    }
    cost_reduction::spells_you_cast_cost_1_less(q, source, player, costed, targets)
}

pub static CARD: CardDef = CardDef { cost_changes: &[spells_you_cast_cost_1_less_during_turns_other_than_yours], ..oracle::GEYSER_DRAKE };
