use cantrip_api::*;
use cantrip_mechanics::provoke;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[provoke::TRIGGER],
    ..oracle::LOWLAND_TRACKER
};
