use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_mechanics::{cost_reduction, effects, learn};
use cantrip_oracle as oracle;

/// "Noncreature spells you cast cost {1} less to cast as long as there are three or more Lesson cards in your graveyard"
/// (CR 601.2f, 118.7a), while the controller's graveyard holds three Lesson cards.
fn noncreature_spells_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if learn::lesson_cards_in_graveyard(q, player) >= 3 { cost_reduction::noncreature_spells_you_cast(q, source, player, costed, 1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, effects::loot, "Whenever Gran-Gran becomes tapped, draw a card, then discard a card.")],
    cost_changes: &[noncreature_spells_cost_one_less],
    ..oracle::GRAN_GRAN
};
