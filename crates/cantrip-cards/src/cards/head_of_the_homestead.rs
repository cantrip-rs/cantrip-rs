use cantrip_api::*;
use cantrip_mechanics::tokens::{self, RABBIT};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| tokens::create(g, item.controller, &RABBIT, 2).map(drop), "When this creature enters, create two 1/1 white Rabbit creature tokens.")],
    ..oracle::HEAD_OF_THE_HOMESTEAD
};

pub static EXTRA: &[&CardDef] = &[&RABBIT];
