use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from_type::<{ CardType::Creature as u8 }>("Protection from creatures")],
    ..oracle::COMMANDER_EESHA
};
