use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature dies during your turn, investigate." (CR 701.16a, 603.2): "during your turn" is part of the trigger
/// event, read as the creature dies, not an intervening "if"; no turn can end before the ability resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, tokens::you_investigate, "When this creature dies during your turn, investigate.")
        .condition(cantrip_mechanics::your_turn)],
    ..oracle::STARFLEET_SECURITY
};
