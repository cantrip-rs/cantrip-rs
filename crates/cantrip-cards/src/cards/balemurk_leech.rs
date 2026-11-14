use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        effects::each_opponent_loses_1_life,
        "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, each opponent loses 1 life.",
    )],
    ..oracle::BALEMURK_LEECH
};
