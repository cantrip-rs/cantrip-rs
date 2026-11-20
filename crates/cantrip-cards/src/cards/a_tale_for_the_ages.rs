use cantrip_api::*;
use cantrip_mechanics::statics::{enchanted_creatures_you_control, plus_pt};
use cantrip_oracle as oracle;

/// "Enchanted creatures you control get +2/+2." (CR 613.4c, layer 7c).
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, enchanted_creatures_you_control, plus_pt::<2, 2>, "Enchanted creatures you control get +2/+2.")],
    ..oracle::A_TALE_FOR_THE_AGES
};
