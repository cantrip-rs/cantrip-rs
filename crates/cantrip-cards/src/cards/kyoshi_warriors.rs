use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, tokens::create_an_ally, "When this creature enters, create a 1/1 white Ally creature token.")],
    ..oracle::KYOSHI_WARRIORS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
