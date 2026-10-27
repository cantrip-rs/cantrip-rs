use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::ability(Subtype::Mountain, mana("{2}"), "Mountaincycling {2} ({2}, Discard this card: Search your library for a Mountain card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::BEDHEAD_BEASTIE
};
