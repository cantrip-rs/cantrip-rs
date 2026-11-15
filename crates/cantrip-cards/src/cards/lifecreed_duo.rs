use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        |g, item| effects::gain_life(g, item, 1),
        "Whenever another creature you control enters, you gain 1 life.",
    )],
    ..oracle::LIFECREED_DUO
};
