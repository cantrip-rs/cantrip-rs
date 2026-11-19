use cantrip_api::*;
use cantrip_mechanics::tokens::{self, GREEN_DINOSAUR};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| tokens::create(g, item.controller, &GREEN_DINOSAUR, 1).map(drop), "When this creature enters, create a 3/3 green Dinosaur creature token."),
        typecycling::ability(Subtype::Forest, mana("{2}"), "Forestcycling {2} ({2}, Discard this card: Search your library for a Forest card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::NURTURING_BRISTLEBACK
};

pub static EXTRA: &[&CardDef] = &[&GREEN_DINOSAUR];
