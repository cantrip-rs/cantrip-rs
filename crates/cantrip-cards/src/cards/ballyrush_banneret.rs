use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    cost_changes: &[cost_reduction::either_subtype_spells_you_cast::<1, { lands::sub(Subtype::Kithkin) }, { lands::sub(Subtype::Soldier) }>],
    ..oracle::BALLYRUSH_BANNERET
};
