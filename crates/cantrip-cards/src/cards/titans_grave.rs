use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Black, Color::Green]), "{T}: Add {B} or {G}."),
        lands::surveil_ability(cost("{2}{B}{G}, {T}"), "{2}{B}{G}, {T}: Surveil 1."),
    ],
    ..oracle::TITANS_GRAVE
};
