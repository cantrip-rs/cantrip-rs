use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target nonland permanent with mana value X" (CR 107.3a, 202.3): X is announced before the target is chosen
/// (`late_targets`) and judged again as the spell resolves. Before it is announced, with the card still in a hand, every
/// nonland permanent is allowed, since some X would allow it.
fn nonland_permanent_with_mana_value_x(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    (targets::NONLAND_PERMANENT.filter)(q, asker, t) && targets::mana_value_is_x(q, id)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(nonland_permanent_with_mana_value_x)], effects::return_first_target_to_hand_then_draw),
    late_targets: true,
    ..oracle::REPEAL
};
