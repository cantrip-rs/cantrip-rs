use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::CASCADE,
        triggers::CASCADE,
        triggers::CASCADE,
        triggers::CASCADE,
    ],
    ..oracle::APEX_DEVASTATOR
};
