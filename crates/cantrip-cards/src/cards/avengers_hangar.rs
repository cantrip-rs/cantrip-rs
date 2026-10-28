use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &lands::gain_land(Color::White, Color::Blue, "{T}: Add {W} or {U}."),
    ..oracle::AVENGERS_HANGAR
};
