use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Plains, mana("{2}"), "Plainscycling {2}"),
        typecycling::ability(Subtype::Island, mana("{2}"), "Islandcycling {2}"),
    ],
    ..oracle::SANCTUM_PLOWBEAST
};
