use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Island, mana("{2}"), "Islandcycling {2}"),
        typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2}"),
    ],
    ..oracle::JHESSIAN_ZOMBIES
};
