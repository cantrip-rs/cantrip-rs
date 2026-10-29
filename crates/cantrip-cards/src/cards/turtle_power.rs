use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::your_of_subtype::<{ lands::sub(Subtype::Turtle) }>,
        plus_pt::<2, 2>,
        "Turtles you control get +2/+2.",
    )],
    ..oracle::TURTLE_POWER
};
