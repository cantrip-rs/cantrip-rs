use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::gain_land(Color::Red, Color::Green, "{T}: Add {R} or {G}."),
    ..oracle::LOS_DIABLOS_MISSILE_BASE
};
