use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This spell costs {3} less to cast if you've gained 3 or more life this turn" (CR 601.2f, 118.7): the life gained this turn by the player casting it,
/// all gains added up, judged as the cost is worked out; life lost does not lessen it.
fn less_after_gaining_three_life(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if q.history().life_gained(player) >= 3 { CostDelta::less(3) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], effects::return_each_target_to_hand),
    own_cost_changes: &[less_after_gaining_three_life],
    ..oracle::SANGUINE_INDULGENCE
};
