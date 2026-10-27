use cantrip_api::*;
use cantrip_mechanics::megamorph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[megamorph::way(Cost::mana(mana("{6}{G}")), "Megamorph {6}{G}")],
    ..oracle::SEGMENTED_KROTIQ
};
