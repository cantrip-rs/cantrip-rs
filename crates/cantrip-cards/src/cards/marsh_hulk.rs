use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{6}{B}")), "Megamorph {6}{B}")],
    ..oracle::MARSH_HULK
};
