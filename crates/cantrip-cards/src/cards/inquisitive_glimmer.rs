use cantrip_api::*;
use cantrip_mechanics::cost_reduction::spells_you_cast;
use cantrip_oracle as oracle;

/// "Enchantment spells you cast cost {1} less to cast" (CR 601.2f, 118.7a).
fn enchantment_spells_cost_1_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    spells_you_cast(q, source, player, costed, CardType::Enchantment, 1)
}

/// "Unlock costs you pay cost {1} less" (CR 709.5e, 116.2m, 118.7a): unlocking a Room's door is a special action, so only a cost
/// change that names it applies (CR 116.2): the rules' own unlock action of a Room, whichever door, and only its source's
/// controller's. A generic reduction, it takes only the generic part of the door's mana cost. The rules' unlock actions are the
/// only special actions of the intrinsic abilities (turning a permanent face up belongs to the face-down values), and no card's
/// own special action sits at their indices, so either half of the test would do alone; both are kept so that neither fact is
/// leaned on.
fn unlock_costs_you_pay_1_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::SpecialAction(_, ability) = costed else { return CostDelta::NONE };
    let unlock = std::ptr::eq(q.card(ability.card), &intrinsic::INTRINSIC) && matches!(ability.index, intrinsic::UNLOCK_LEFT | intrinsic::UNLOCK_RIGHT);
    if unlock && player == q.controller(source) { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[enchantment_spells_cost_1_less, unlock_costs_you_pay_1_less],
    ..oracle::INQUISITIVE_GLIMMER
};
