use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, lands, targets, you_control_an_enchantment};
use cantrip_oracle as oracle;

/// "It also costs {1} less to cast if you control an enchantment" (CR 601.2f): a separate reduction from the Spirit one, so one
/// permanent that is both a Spirit and an enchantment makes each apply (the {2} the ruling gives).
fn less_with_an_enchantment(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if you_control_an_enchantment(q, player) { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effects::counter_first_target_unless_pays::<3>),
    own_cost_changes: &[cost_reduction::less_if_you_control_subtype::<1, { lands::sub(Subtype::Spirit) }>, less_with_an_enchantment],
    ..oracle::GEISTLIGHT_SNARE
};
