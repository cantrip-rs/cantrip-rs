use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        protection::from_subtype::<{ Subtype::Zombie as u16 }>("Protection from Zombies"),
    ],
    ..oracle::GRAVE_BRAMBLE
};
