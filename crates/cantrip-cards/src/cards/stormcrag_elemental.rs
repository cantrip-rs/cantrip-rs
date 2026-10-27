use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{4}{R}{R}")), "Megamorph {4}{R}{R}")],
    ..oracle::STORMCRAG_ELEMENTAL
};
