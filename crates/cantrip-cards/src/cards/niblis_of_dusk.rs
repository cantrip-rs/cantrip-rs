use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
    ],
    ..oracle::NIBLIS_OF_DUSK
};
