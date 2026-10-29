use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::Blue, Color::Red]), "{T}: Add {U} or {R}."),
        lands::surveil_ability(cost("{2}{U}{R}, {T}"), "{2}{U}{R}, {T}: Surveil 1."),
    ],
    ..oracle::SPECTACLE_SUMMIT
};
