use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effects::damage_first_target::<4>),
    ..oracle::FLAME_LASH
};
