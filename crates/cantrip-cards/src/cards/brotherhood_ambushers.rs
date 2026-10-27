use cantrip_api::*;
use cantrip_mechanics::freerunning;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[freerunning::way(mana("{3}{B}"), "Freerunning {3}{B}")], ..oracle::BROTHERHOOD_AMBUSHERS };
