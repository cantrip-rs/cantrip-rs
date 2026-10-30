use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[lands::SACRIFICE_TO_FETCH_BASIC_TAPPED],
    ..oracle::EVOLVING_WILDS
};
