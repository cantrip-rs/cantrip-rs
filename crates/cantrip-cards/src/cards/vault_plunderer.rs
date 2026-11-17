use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, target player draws a card and loses 1 life." (CR 121.1, 119.3): any player, its own controller
/// included, as the target.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::target_player_draws_and_loses_life::<1, 1>, "When this creature enters, target player draws a card and loses 1 life.")
        .targets(&[targets::PLAYER])],
    ..oracle::VAULT_PLUNDERER
};
