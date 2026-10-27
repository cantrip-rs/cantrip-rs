use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::EXALTED,
    ],
    ..oracle::OUTRIDER_OF_JHESS
};
