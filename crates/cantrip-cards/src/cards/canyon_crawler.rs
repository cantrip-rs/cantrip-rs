use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{tokens, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::CANYON_CRAWLER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
