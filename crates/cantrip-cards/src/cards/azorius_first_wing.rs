use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from_type::<{ CardType::Enchantment as u8 }>("Protection from enchantments")],
    ..oracle::AZORIUS_FIRST_WING
};
