use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{4}{R}")), "Morph {4}{R}")],
    ..oracle::AINOK_TRACKER
};
