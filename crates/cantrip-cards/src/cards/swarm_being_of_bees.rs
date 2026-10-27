use cantrip_api::*;
use cantrip_mechanics::mayhem;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[mayhem::way(mana("{B}"), "Mayhem {B}")],
    ..oracle::SWARM_BEING_OF_BEES
};
