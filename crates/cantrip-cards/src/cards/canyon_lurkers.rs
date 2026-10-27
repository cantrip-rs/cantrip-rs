use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{3}{R}")), "Morph {3}{R}")],
    ..oracle::CANYON_LURKERS
};
