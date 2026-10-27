use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{U}")), "Morph {U}")],
    ..oracle::MONASTERY_FLOCK
};
