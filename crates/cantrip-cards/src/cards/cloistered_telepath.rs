use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The creature entering is the trigger event (CR 603.6a); `loot` performs the draw followed by the discard.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::loot,
        "When this creature enters, draw a card, then discard a card.",
    )],
    ..oracle::CLOISTERED_TELEPATH
};
