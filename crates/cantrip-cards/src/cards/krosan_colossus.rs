use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{6}{G}{G}")), "Morph {6}{G}{G}")],
    ..oracle::KROSAN_COLOSSUS
};
