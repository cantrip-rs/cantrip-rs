use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::creature_or_planeswalker_of_either_color::<{ Color::Black as u8 }, { Color::Red as u8 }>()], effects::exile_first_target_then_scry::<1>),
    ..oracle::DEVOUT_DECREE
};
