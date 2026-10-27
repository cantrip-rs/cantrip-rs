use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::Red as u8 }>("Protection from red")],
    ..oracle::NARWHAL
};
