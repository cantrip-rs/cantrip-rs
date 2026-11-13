use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE, targets::LAND], effects::destroy_each_target),
    additional_cost: Some(life(3)),
    ..oracle::FUMAROLE
};
