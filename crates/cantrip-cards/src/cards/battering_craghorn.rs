use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{1}{R}{R}")), "Morph {1}{R}{R}")],
    ..oracle::BATTERING_CRAGHORN
};
