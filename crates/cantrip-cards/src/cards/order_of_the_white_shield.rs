use cantrip_api::*;
use cantrip_mechanics::{effects, protection};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::Black as u8 }>("Protection from black")],
    abilities: &[
        ability(cost("{W}"), effects::this_gains_first_strike, "{W}: This creature gains first strike until end of turn."),
        ability(cost("{W}{W}"), effects::this_gets::<1, 0>, "{W}{W}: This creature gets +1/+0 until end of turn."),
    ],
    ..oracle::ORDER_OF_THE_WHITE_SHIELD
};
