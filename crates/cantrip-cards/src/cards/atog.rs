use cantrip_api::*;
use cantrip_mechanics::{effects, targets::filters};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, filters::artifact),
        effects::pump_self::<2, 2>,
        "Sacrifice an artifact: This creature gets +2/+2 until end of turn.",
    )],
    ..oracle::ATOG
};
