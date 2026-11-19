use cantrip_api::*;
use cantrip_mechanics::{bargain, cost_reduction, effects, targets};
use cantrip_oracle as oracle;

/// "Bargain. This spell costs {1} less to cast if it's bargained. Counter target spell." (CR 702.166a, 601.2f, 701.6): the sacrifice
/// is announced with the cost and the reduction applies once it is paid; the spell is countered only while its target is still legal.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effects::counter_first_target),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    own_cost_changes: &[cost_reduction::less_if_optional_cost_paid::<1>],
    ..oracle::ICE_OUT
};
