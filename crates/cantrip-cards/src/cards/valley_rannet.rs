use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Mountain, mana("{2}"), "Mountaincycling {2}"),
        typecycling::ability(Subtype::Forest, mana("{2}"), "Forestcycling {2}"),
    ],
    ..oracle::VALLEY_RANNET
};
