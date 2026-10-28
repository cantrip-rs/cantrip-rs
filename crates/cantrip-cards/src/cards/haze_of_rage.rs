use cantrip_api::*;
use cantrip_mechanics::{effects, storm};
use cantrip_mechanics::replacements::BUYBACK;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::creatures_you_control_get::<1, 0>),
    kicker: kicker(mana("{2}")),
    replacements: &[BUYBACK],
    abilities: &[storm::trigger("Storm (When you cast Haze of Rage, copy it for each spell cast before it this turn.)")],
    ..oracle::HAZE_OF_RAGE
};
