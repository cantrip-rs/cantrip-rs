use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[morph::way(Cost::mana(mana("{2}{G}{U}{R}")), "Morph {2}{G}{U}{R}")], ..oracle::SNOWHORN_RIDER };
