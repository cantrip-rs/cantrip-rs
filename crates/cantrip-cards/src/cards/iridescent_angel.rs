use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        protection::from_each_color("Protection from each color"),
    ],
    ..oracle::IRIDESCENT_ANGEL
};
