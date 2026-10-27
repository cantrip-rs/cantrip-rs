use cantrip_api::*;
use cantrip_mechanics::devoid;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    colors: devoid::COLORS,
    ..oracle::VESTIGE_OF_EMRAKUL
};
