use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { ways: &[morph::way(Cost::mana(mana("{5}{G}")), "Morph {5}{G}")], ..oracle::WOOLLY_LOXODON };
