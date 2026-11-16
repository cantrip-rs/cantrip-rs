use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        tokens::create_a_mercenary,
        "When this creature dies, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
    )],
    ..oracle::WANTED_GRIFFIN
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
