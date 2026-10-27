use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        typecycling::basic_landcycling(mana("{1}{G}"), "Basic landcycling {1}{G} ({1}{G}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::TOPIARY_PANTHER
};
