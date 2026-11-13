use cantrip_api::*;
use cantrip_mechanics::cost_reduction;
use cantrip_mechanics::start_your_engines::has_max_speed;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Max speed \u{2014} Spells you cast cost {1} less to cast" (CR 702.178a, 601.2f): as long as its controller's speed is 4, their
/// spells cost {1} less; below that the permanent changes no cost.
fn spells_cost_1_less_at_max_speed(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    if has_max_speed(q.state(), q.controller(source)) {
        cost_reduction::spells_you_cast_cost_1_less(q, source, player, costed, targets)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(THIS_ENTERS, effects::draw_two_then_discard, "When this artifact enters, draw two cards, then discard a card.")],
    cost_changes: &[spells_cost_1_less_at_max_speed],
    ..oracle::RACERS_SCOREBOARD
};
