use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::Blue as u8 }>("Protection from blue")],
    ..oracle::VOICE_OF_REASON
};
