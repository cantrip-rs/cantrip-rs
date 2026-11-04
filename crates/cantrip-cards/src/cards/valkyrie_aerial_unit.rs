use cantrip_api::*;
use cantrip_mechanics::{affinity, effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flying is a core keyword. "Affinity for artifacts" (CR 702.41) and "when this creature enters, surveil 2" (CR 701.25).
pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifacts],
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::surveil(g, item, 2),
        "When this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)",
    )],
    ..oracle::VALKYRIE_AERIAL_UNIT
};
