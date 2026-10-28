use cantrip_api::*;
use cantrip_mechanics::{effects, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}"), effects::this_gets::<1, -1>, "{2}: This creature gets +1/-1 until end of turn."),
        typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2}"),
    ],
    ..oracle::UNDULATING_WITNESS
};
