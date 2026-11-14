use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        |g, item| tokens::create(g, item.controller, &tokens::GLIMMER, 1).map(|_| ()),
        "When this creature enters, create a 1/1 white Glimmer enchantment creature token.",
    )],
    ..oracle::TUNNEL_SURVEYOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::GLIMMER];
