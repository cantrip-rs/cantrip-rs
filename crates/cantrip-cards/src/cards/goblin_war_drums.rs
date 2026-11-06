use cantrip_api::*;
use cantrip_mechanics::statics::{have_menace, your_creatures};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, your_creatures, have_menace, "Creatures you control have menace.")],
    ..oracle::GOBLIN_WAR_DRUMS
};
