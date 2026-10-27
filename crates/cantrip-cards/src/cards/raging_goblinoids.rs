use cantrip_api::*;
use cantrip_mechanics::mayhem;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[mayhem::way(mana("{2}{R}"), "Mayhem {2}{R}")],
    ..oracle::RAGING_GOBLINOIDS
};
