use cantrip_api::*;
use cantrip_mechanics::{effects, protection};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::White as u8 }>("Protection from white")],
    abilities: &[
        ability(cost("{B}"), effects::this_gains_first_strike, "{B}: This creature gains first strike until end of turn."),
        ability(cost("{B}{B}"), effects::this_gets::<1, 0>, "{B}{B}: This creature gets +1/+0 until end of turn."),
    ],
    ..oracle::KNIGHT_OF_STROMGALD
};
