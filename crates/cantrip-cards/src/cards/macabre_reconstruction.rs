use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if a creature card was put into your graveyard from anywhere this turn" (CR 601.2f, 118.7):
/// judged as the cost is determined, for casting this spell only; a creature card of yours that went to your graveyard
/// from any zone counts, wherever it is now, and a token or a card that went to an opponent's graveyard does not.
fn costs_2_less_if_a_creature_card_was_put_into_your_graveyard(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if costed == Costed::Spell(source) && history::creature_cards_put_into_graveyards_this_turn(q, Some(player)) >= 1 {
        CostDelta::less(2)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], effects::return_each_target_to_hand),
    own_cost_changes: &[costs_2_less_if_a_creature_card_was_put_into_your_graveyard],
    ..oracle::MACABRE_RECONSTRUCTION
};
