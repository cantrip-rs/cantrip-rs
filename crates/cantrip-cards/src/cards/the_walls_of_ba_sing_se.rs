use cantrip_api::*;
use cantrip_mechanics::statics::{have_indestructible, other_permanents_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, other_permanents_you_control, have_indestructible, "Other permanents you control have indestructible.")],
    ..oracle::THE_WALLS_OF_BA_SING_SE
};
