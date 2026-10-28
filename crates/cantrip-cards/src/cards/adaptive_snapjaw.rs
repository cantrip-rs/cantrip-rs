use cantrip_api::*;
use cantrip_mechanics::evolve;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[evolve::EVOLVE],
    ..oracle::ADAPTIVE_SNAPJAW
};
