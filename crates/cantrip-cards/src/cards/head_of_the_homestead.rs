use cantrip_api::*;
use cantrip_mechanics::tokens::{self, RABBIT};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_rabbits::<2>, "When this creature enters, create two 1/1 white Rabbit creature tokens.")],
    ..oracle::HEAD_OF_THE_HOMESTEAD
};

pub static EXTRA: &[&CardDef] = &[&RABBIT];
