use cantrip_api::*;
use cantrip_mechanics::surge;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[surge::way(mana("{2}{U}"), "Surge {2}{U}")], ..oracle::JWAR_ISLE_AVENGER };
