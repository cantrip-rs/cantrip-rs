use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

const TEXT: &str = "Other Kobold creatures you control get +0/+1 and have trample.";

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Kobold) }>, statics::have_trample, TEXT),
        static_ability(Layer::PtModify, statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Kobold) }>, statics::plus_pt::<0, 1>, TEXT).continues(),
    ],
    ..oracle::KOBOLD_DRILL_SERGEANT
};
