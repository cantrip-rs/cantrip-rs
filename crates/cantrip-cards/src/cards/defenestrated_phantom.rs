use cantrip_api::*;
use cantrip_mechanics::disguise;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{4}{W}")), "Disguise {4}{W}")],
    ..oracle::DEFENESTRATED_PHANTOM
};
