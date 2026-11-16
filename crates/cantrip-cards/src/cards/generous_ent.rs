use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        typecycling::ability(Subtype::Forest, mana("{1}"), "Forestcycling {1} ({1}, Discard this card: Search your library for a Forest card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::GENEROUS_ENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
