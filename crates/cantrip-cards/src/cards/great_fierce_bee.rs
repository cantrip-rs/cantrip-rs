use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more other creatures die" (CR 700.4, 603.2c): any creature but this one put into a graveyard from the
/// battlefield, asked of each creature of the batch, and one trigger for the creatures that die together. This creature
/// dying with them still sees them die (CR 603.10a).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, triggers::another_creature_dies), |g, item| effects::scry(g, item, 1), "Whenever one or more other creatures die, scry 1.")
        .once_per_batch()],
    ..oracle::GREAT_FIERCE_BEE
};
