use cantrip_api::*;
use cantrip_mechanics::disguise;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{4}{R/G}{R/G}")), "Disguise {4}{R/G}{R/G}")],
    ..oracle::RIFTBURST_HELLION
};
