use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{3}{W}")), "Morph {3}{W}")],
    ..oracle::SAGE_EYE_HARRIER
};
