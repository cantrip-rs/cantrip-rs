use cantrip_api::*;
use cantrip_mechanics::morph;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[morph::way(Cost::mana(mana("{2}{W}{B}{G}")), "Morph {2}{W}{B}{G}")],
    ..oracle::ABZAN_GUIDE
};
