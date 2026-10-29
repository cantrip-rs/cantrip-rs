use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_LEAVES};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_LEAVES,
        |g, item| {
            tokens::create_food(g, item.controller)?;
            Ok(())
        },
        "When this creature leaves the battlefield, create a Food token.",
    )],
    ..oracle::FEATHERBRAINED_FILCHER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
