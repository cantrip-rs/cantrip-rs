use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Green, Color::Blue]), "{T}: Add {G} or {U}."),
        lands::surveil_ability(cost("{2}{G}{U}, {T}"), "{2}{G}{U}, {T}: Surveil 1."),
    ],
    ..oracle::PARADOX_GARDENS
};
