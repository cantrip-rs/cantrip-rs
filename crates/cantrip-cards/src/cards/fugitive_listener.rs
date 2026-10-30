use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, each opponent discards a card." (CR 603.6a, 701.9a, 101.4).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::each_opponent_discards_a_card,
        "When this creature enters, each opponent discards a card.",
    )],
    ..oracle::FUGITIVE_LISTENER
};
