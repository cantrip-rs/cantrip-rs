use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[morph::way(Cost::mana(mana("{2}{B}")), "Morph {2}{B}")], ..oracle::SPINED_BASHER };
