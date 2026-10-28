use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::gain_land(Color::Black, Color::Green, "{T}: Add {B} or {G}."),
    ..oracle::SUBTERRANEAN_CAVERN
};
