use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        protection::from::<{ Color::Red as u8 }>("Protection from red"),
        protection::from::<{ Color::Green as u8 }>("Protection from green"),
    ],
    ..oracle::SPHINX_OF_THE_STEEL_WIND
};
