use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, lands};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if you control a Wizard." (CR 601.2f, 118.7a) then "Draw three cards." (CR 121.1).
pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cost_reduction::less_if_you_control_subtype::<1, { lands::sub(Subtype::Wizard) }>],
    spell: spell(&[], |g, item| effects::draw(g, item, 3)),
    ..oracle::ARCANE_EPIPHANY
};
