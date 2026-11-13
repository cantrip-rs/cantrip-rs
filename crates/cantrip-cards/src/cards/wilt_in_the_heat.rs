use cantrip_api::*;
use cantrip_mechanics::history::cards_left_graveyard_this_turn;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if a card left your graveyard this turn" (CR 601.2f, 700.4): judged once, as the total cost
/// is determined, from the event history, and generic mana only (CR 118.7a).
fn less_if_a_card_left_your_graveyard(q: &dyn Query, _: ObjectId, caster: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if cards_left_graveyard_this_turn(q, caster) > 0 { CostDelta::less(2) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::damage_target_exile_if_it_would_die::<5>),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    own_cost_changes: &[less_if_a_card_left_your_graveyard],
    ..oracle::WILT_IN_THE_HEAT
};
