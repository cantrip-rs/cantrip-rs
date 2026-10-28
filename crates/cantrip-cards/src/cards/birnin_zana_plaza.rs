use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::gain_land(Color::Green, Color::White, "{T}: Add {G} or {W}."),
    ..oracle::BIRNIN_ZANA_PLAZA
};
