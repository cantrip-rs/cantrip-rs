use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::ping_land(Color::Red, Color::Green, "{T}: Add {R} or {G}."),
    ..oracle::BRISTLING_BACKWOODS
};
