use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::have_double_strike;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[static_ability(Layer::Ability, attached_object, have_double_strike, "Equipped creature has double strike.")],
    ..oracle::HARD_WON_JITTE
};
