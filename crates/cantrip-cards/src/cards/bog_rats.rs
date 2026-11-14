use cantrip_api::*;
use cantrip_mechanics::{combat, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    block_rules: &[combat::cant_be_blocked_by_subtype::<{ lands::sub(Subtype::Wall) }>],
    ..oracle::BOG_RATS
};
