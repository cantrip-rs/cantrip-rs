use cantrip_api::*;
use cantrip_mechanics::extort;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[extort::EXTORT],
    ..oracle::SYNDICATE_ENFORCER
};
