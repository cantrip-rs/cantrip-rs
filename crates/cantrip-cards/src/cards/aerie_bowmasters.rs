use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{5}{G}")), "Megamorph {5}{G}")],
    ..oracle::AERIE_BOWMASTERS
};
