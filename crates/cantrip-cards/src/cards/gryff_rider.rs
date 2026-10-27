use cantrip_api::*;
use cantrip_mechanics::training;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        training::TRIGGER,
    ],
    ..oracle::GRYFF_RIDER
};
