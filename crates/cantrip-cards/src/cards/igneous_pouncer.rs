use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2}"),
        typecycling::ability(Subtype::Mountain, mana("{2}"), "Mountaincycling {2}"),
    ],
    ..oracle::IGNEOUS_POUNCER
};
