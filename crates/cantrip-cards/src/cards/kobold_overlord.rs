use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Kobold) }>, statics::have_first_strike, "Other Kobold creatures you control have first strike.")],
    ..oracle::KOBOLD_OVERLORD
};
