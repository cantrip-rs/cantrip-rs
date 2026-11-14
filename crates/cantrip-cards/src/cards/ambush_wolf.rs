use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, exile up to one target card from a graveyard." (CR 603.6a, 115.1, 701.13a): the target is "up to one" (CR 601.2c), so it may be chosen as none.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::exile_first_target,
        "When this creature enters, exile up to one target card from a graveyard.",
    )
    .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()])],
    ..oracle::AMBUSH_WOLF
};
