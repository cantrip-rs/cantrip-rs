use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Dinosaur) }>,
        plus_pt::<1, 1>,
        "Other Dinosaurs you control get +1/+1.",
    )],
    ..oracle::REGAL_IMPERIOSAUR
};
