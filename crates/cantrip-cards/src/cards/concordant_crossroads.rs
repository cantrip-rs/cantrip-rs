use cantrip_api::*;
use cantrip_mechanics::statics::{all_creatures, have_haste};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, all_creatures, have_haste, "All creatures have haste.")],
    ..oracle::CONCORDANT_CROSSROADS
};
