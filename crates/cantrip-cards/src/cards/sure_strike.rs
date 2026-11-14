use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_first_strike::<3, 0>),
    ..oracle::SURE_STRIKE
};
