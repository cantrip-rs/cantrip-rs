use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| library::mill_then_may_take_land_or_grow(g, item, 3),
        "When this creature enters, mill three cards. You may put a land card from among them into your hand. If you don't, put a +1/+1 counter on this creature. (To mill three cards, put the top three cards of your library into your graveyard.)",
    )],
    ..oracle::OSTRICH_HORSE
};
