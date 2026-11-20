use cantrip_api::*;
use cantrip_mechanics::{bargain, cost_reduction, effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    own_cost_changes: &[cost_reduction::less_if_optional_cost_paid::<2>],
    abilities: &[triggered(THIS_ENTERS, effects::gain_life_amount::<3>, "When this creature enters, you gain 3 life.")],
    ..oracle::HAMLET_GLUTTON
};
