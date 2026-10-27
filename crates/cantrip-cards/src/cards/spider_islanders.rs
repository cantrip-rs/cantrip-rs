use cantrip_api::*;
use cantrip_mechanics::mayhem;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[mayhem::way(mana("{1}{R}"), "Mayhem {1}{R}")],
    ..oracle::SPIDER_ISLANDERS
};
