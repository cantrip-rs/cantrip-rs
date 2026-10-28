use cantrip_api::*;
use cantrip_mechanics::{devoid, ingest};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    colors: devoid::COLORS,
    abilities: &[ingest::TRIGGER],
    ..oracle::CULLING_DRONE
};
