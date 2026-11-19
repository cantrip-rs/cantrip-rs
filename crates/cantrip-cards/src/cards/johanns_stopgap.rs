use cantrip_api::*;
use cantrip_mechanics::{bargain, cost_reduction, effects, targets};
use cantrip_oracle as oracle;

/// "Return target nonland permanent to its owner's hand. Draw a card." (CR 400.3, 121.1, 608.2b): with its only target illegal the
/// spell does not resolve (the resolver fizzles it before this runs), so no card is drawn (the ruling); otherwise the draw
/// follows whether or not the permanent moved.
fn return_it_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    g.end_sentence();
    effects::draw(g, item, 1)
}

/// "Bargain. This spell costs {2} less to cast if it's bargained." (CR 702.166a, 601.2f): its mana value stays 4 (CR 202.3).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], return_it_then_draw),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    own_cost_changes: &[cost_reduction::less_if_optional_cost_paid::<2>],
    ..oracle::JOHANNS_STOPGAP
};
