use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

/// "Protection from black" (CR 702.16a); defender is in the Oracle characteristics.
pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::Black as u8 }>("Protection from black")],
    ..oracle::WALL_OF_LIGHT
};
