use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, target player mills two cards." (CR 701.17a, 608.2b): any player may be the target, the one
/// who controls it included, and nothing happens if the player is no longer a legal target as the ability resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, effects::target_player_mills::<2>, "When this creature enters, target player mills two cards.")
        .targets(&[targets::PLAYER])],
    ..oracle::DESPERATE_BLOODSEEKER
};
