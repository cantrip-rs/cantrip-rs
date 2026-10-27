use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{1}{W}")), "Megamorph {1}{W}")],
    ..oracle::MISTHOOF_KIRIN
};
