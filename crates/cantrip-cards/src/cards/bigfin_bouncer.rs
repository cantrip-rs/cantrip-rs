use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, return target creature an opponent controls to its owner's hand." (CR 603.6a, 400.3).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return target creature an opponent controls to its owner's hand.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::BIGFIN_BOUNCER
};
