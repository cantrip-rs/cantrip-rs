use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Kobold) }>, statics::plus_pt::<1, 0>, "Other Kobold creatures you control get +1/+0.")],
    ..oracle::KOBOLD_TASKMASTER
};
