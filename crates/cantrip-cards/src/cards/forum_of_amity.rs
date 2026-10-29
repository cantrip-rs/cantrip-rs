use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::White, Color::Black]), "{T}: Add {W} or {B}."),
        lands::surveil_ability(cost("{2}{W}{B}, {T}"), "{2}{W}{B}, {T}: Surveil 1."),
    ],
    ..oracle::FORUM_OF_AMITY
};
