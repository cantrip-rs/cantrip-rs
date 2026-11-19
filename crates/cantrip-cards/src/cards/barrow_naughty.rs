use cantrip_api::*;
use cantrip_mechanics::statics::{have_lifelink, itself_while_you_control_another};
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{B}"), effects::this_gets::<1, 0>, "{2}{B}: This creature gets +1/+0 until end of turn.")],
    statics: &[static_ability(Layer::Ability, itself_while_you_control_another::<{ lands::sub(Subtype::Faerie) }>, have_lifelink, "This creature has lifelink as long as you control another Faerie.")],
    ..oracle::BARROW_NAUGHTY
};
