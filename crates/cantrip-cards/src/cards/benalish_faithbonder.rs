use cantrip_api::*;
use cantrip_mechanics::enlist;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        enlist::ENLIST,
    ],
    ..oracle::BENALISH_FAITHBONDER
};
