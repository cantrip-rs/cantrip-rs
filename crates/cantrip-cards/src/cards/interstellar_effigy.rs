use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, return target creature an opponent controls to its owner's hand."
/// Target legality is checked when the trigger is put on the stack and again when it resolves (CR 603.3d, 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_hand,
        "When this creature enters, return target creature an opponent controls to its owner's hand.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::INTERSTELLAR_EFFIGY
};
