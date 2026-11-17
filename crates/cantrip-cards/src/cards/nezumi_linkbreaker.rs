use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_DIES};
use cantrip_oracle as oracle;

/// "When this creature dies, create a 1/1 red Mercenary creature token with ..." (CR 700.4, 111.1): the token comes under the
/// control of the creature's controller as it died (CR 603.3a, 603.10a), whoever killed it.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        tokens::create_a_mercenary,
        "When this creature dies, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
    )],
    ..oracle::NEZUMI_LINKBREAKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
