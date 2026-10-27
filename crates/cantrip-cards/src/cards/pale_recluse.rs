use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Forest, mana("{2}"), "Forestcycling {2}"),
        typecycling::ability(Subtype::Plains, mana("{2}"), "Plainscycling {2}"),
    ],
    ..oracle::PALE_RECLUSE
};
