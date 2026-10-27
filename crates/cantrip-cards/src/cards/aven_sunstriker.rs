use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{4}{W}")), "Megamorph {4}{W}")],
    ..oracle::AVEN_SUNSTRIKER
};
