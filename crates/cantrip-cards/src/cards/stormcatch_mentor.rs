use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, triggers::PROWESS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[PROWESS],
    cost_changes: &[cost_reduction::instants_and_sorceries_you_cast_cost_less::<1>],
    ..oracle::STORMCATCH_MENTOR
};
