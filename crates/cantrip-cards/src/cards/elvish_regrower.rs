use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, return target permanent card from your graveyard to your hand." (CR 603.6a, 115.1, 400.3): a
/// permanent card is a card of a permanent type (CR 110.4), any of the player's own graveyard's.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_hand,
        "When this creature enters, return target permanent card from your graveyard to your hand.",
    )
    .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::ELVISH_REGROWER
};
