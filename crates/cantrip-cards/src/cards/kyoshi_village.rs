use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::sacrifice_draw_land(Color::Green, Color::White, "{T}: Add {G} or {W}."),
    ..oracle::KYOSHI_VILLAGE
};
