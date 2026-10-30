use cantrip_api::*;
use cantrip_mechanics::lands::ENTERS_MAY_FETCH_BASIC_TO_HAND;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ENTERS_MAY_FETCH_BASIC_TO_HAND],
    ..oracle::ENVIRONMENTAL_SCIENTIST
};
