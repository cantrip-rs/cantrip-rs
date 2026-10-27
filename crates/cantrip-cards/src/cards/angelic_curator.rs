use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from_type::<{ CardType::Artifact as u8 }>("Protection from artifacts")],
    ..oracle::ANGELIC_CURATOR
};
