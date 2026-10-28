use cantrip_api::*;
use cantrip_mechanics::{escape, targets, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[typecycling::basic_landcycling(mana("{2}"), "Basic landcycling {2} ({2}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)")],
    ways: &[escape::way_exiling_permanent(
        mana("{4}{G}{U}"),
        5,
        targets::filters::land,
        "Escape\u{2014}{4}{G}{U}, Exile a land you control, Exile five other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)",
    )],
    ..oracle::LUNAR_HATCHLING
};
