use cantrip_api::*;
use cantrip_mechanics::{modular, sunburst};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(sunburst::enters),
    abilities: &[modular::TRIGGER],
    ..oracle::ARCBOUND_WANDERER
};
