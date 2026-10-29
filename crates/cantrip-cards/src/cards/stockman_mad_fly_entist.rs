use cantrip_api::*;
use cantrip_mechanics::effects::loot;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, loot, "When Stockman enters, draw a card, then discard a card."),
        typecycling::ability(Subtype::Island, mana("{2}"), "Islandcycling {2} ({2}, Discard this card: Search your library for an Island card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::STOCKMAN_MAD_FLY_ENTIST
};
