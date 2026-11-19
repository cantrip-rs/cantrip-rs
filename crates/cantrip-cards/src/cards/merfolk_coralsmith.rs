use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), effects::this_gets::<1, -1>, "{1}: This creature gets +1/-1 until end of turn."),
        triggered(triggers::THIS_DIES, effects::scry_two, "When this creature dies, scry 2."),
    ],
    ..oracle::MERFOLK_CORALSMITH
};
