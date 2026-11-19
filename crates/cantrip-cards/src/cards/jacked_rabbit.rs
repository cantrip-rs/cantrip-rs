use cantrip_api::*;
use cantrip_mechanics::{ravenous, tokens::{self, RABBIT}};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(ravenous::enters),
    abilities: &[
        ravenous::draw_trigger("When Jacked Rabbit enters, if X is 5 or more, draw a card."),
        triggered(
            cantrip_mechanics::triggers::THIS_ATTACKS,
            |g, item| tokens::create_equal_to_its_power(g, item, &RABBIT),
            "Whenever Jacked Rabbit attacks, create a number of 1/1 white Rabbit creature tokens equal to its power.",
        ),
    ],
    ..oracle::JACKED_RABBIT
};

pub static EXTRA: &[&CardDef] = &[&RABBIT];
