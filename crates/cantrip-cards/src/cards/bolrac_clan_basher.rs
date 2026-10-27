use cantrip_api::*;
use cantrip_mechanics::disguise;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{3}{R}{R}")), "Disguise {3}{R}{R}")],
    ..oracle::BOLRAC_CLAN_BASHER
};
