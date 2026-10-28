use cantrip_api::*;
use cantrip_mechanics::mentor;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mentor::MENTOR],
    ..oracle::SUNHOME_STALWART
};
