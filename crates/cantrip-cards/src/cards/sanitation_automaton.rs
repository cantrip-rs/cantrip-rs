use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, surveil 1." (CR 603.6a, 701.25a): look at the top card of the library, then put it into the
/// graveyard or leave it on top.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::surveil(g, item, 1), "When this creature enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)")],
    ..oracle::SANITATION_AUTOMATON
};
