use cantrip_api::*;
use cantrip_mechanics::flanking;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[flanking::TRIGGER],
    ..oracle::BENALISH_CAVALRY
};
