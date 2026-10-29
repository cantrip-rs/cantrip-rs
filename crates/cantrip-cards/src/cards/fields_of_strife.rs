use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Red, Color::White]), "{T}: Add {R} or {W}."),
        lands::surveil_ability(cost("{2}{R}{W}, {T}"), "{2}{R}{W}, {T}: Surveil 1."),
    ],
    ..oracle::FIELDS_OF_STRIFE
};
