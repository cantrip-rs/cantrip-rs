use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    cost_changes: &[cost_reduction::subtype_spells_you_cast::<1, { lands::sub(Subtype::Dragon) }>],
    ..oracle::DRAGONLORDS_SERVANT
};
