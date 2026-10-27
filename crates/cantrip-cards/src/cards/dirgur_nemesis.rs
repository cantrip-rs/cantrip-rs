use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{6}{U}")), "Megamorph {6}{U}")],
    ..oracle::DIRGUR_NEMESIS
};
