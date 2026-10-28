use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cycle or discard a card": cycling discards the card (CR 702.29a), so its batch holds a discard event
/// and the one trigger fires once for a cycled card (CR 702.29d), as for any discard.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_DISCARD, effects::this_gets::<1, 0>, "Whenever you cycle or discard a card, this creature gets +1/+0 until end of turn.")],
    ..oracle::FLAMEBLADE_ADEPT
};
