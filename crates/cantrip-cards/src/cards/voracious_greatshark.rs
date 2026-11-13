use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flash comes from the card. "When this creature enters, counter target artifact or creature spell." (CR 603.6a, 701.6a): the target
/// is chosen as the trigger goes on the stack, so only a spell already on it, cast before this creature, can be one; the creature
/// itself, as it entered, is not a spell (CR 112.1).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::counter_first_target, "When this creature enters, counter target artifact or creature spell.")
        .targets(&[targets::ARTIFACT_OR_CREATURE_SPELL])],
    ..oracle::VORACIOUS_GREATSHARK
};
