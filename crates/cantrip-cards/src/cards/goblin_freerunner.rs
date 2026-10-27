use cantrip_api::*;
use cantrip_mechanics::surge;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[surge::way(mana("{1}{R}"), "Surge {1}{R}")], ..oracle::GOBLIN_FREERUNNER };
