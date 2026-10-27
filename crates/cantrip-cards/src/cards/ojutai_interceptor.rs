use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{3}{U}")), "Megamorph {3}{U}")],
    ..oracle::OJUTAI_INTERCEPTOR
};
