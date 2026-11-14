use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flying is a core keyword. "When this creature enters, exile target player's graveyard" (CR 115.1, 406.2): any player, its own
/// controller included.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::exile_first_target_players_graveyard, "When this creature enters, exile target player's graveyard.")
        .targets(&[targets::PLAYER])],
    ..oracle::ANGEL_OF_FINALITY
};
