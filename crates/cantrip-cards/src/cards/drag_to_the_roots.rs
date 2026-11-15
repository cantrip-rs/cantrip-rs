use cantrip_api::*;
use cantrip_mechanics::{delirium, effects, targets};
use cantrip_oracle as oracle;

/// "Delirium \u{2014} This spell costs {2} less to cast as long as there are four or more card types among cards in your graveyard"
/// (CR 601.2f, 118.7a, 207.2c): judged once, as the total cost is determined, and generic mana only, so the two colored symbols stay.
fn two_less_with_delirium(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if delirium(q, player) { CostDelta::less(2) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], effects::destroy_first_target),
    own_cost_changes: &[two_less_with_delirium],
    ..oracle::DRAG_TO_THE_ROOTS
};
