use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[morph::way(Cost::mana(mana("{4}{G}{G}{G}")), "Morph {4}{G}{G}{G}")], ..oracle::TITANIC_BULVOX };
