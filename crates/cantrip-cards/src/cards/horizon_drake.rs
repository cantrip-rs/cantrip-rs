use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from_type::<{ CardType::Land as u8 }>("Protection from lands")],
    ..oracle::HORIZON_DRAKE
};
