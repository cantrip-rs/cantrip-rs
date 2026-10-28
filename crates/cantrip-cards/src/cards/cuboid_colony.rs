use cantrip_api::*;
use cantrip_mechanics::increment;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[increment::TRIGGER],
    ..oracle::CUBOID_COLONY
};
