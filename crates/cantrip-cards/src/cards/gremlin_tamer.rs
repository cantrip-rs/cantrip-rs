use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        tokens::create_a_gremlin,
        "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, create a 1/1 red Gremlin creature token.",
    )],
    ..oracle::GREMLIN_TAMER
};

pub static EXTRA: &[&CardDef] = &[&tokens::GREMLIN];
