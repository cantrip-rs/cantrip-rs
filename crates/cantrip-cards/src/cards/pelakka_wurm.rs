use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::{THIS_DIES, THIS_ENTERS}};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::gain_life(g, item, 7), "When this creature enters, you gain 7 life."),
        triggered(THIS_DIES, |g, item| effects::draw(g, item, 1), "When this creature dies, draw a card."),
    ],
    ..oracle::PELAKKA_WURM
};
