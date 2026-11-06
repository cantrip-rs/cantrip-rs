use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, targets::filters::creature),
        effects::pump_self::<2, 1>,
        "Sacrifice a creature: This creature gets +2/+1 until end of turn.",
    )],
    ..oracle::FALLEN_ANGEL
};
