use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, tokens::create_an_ally, "When this creature dies, create a 1/1 white Ally creature token.")],
    ..oracle::PRETENDING_POXBEARERS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
