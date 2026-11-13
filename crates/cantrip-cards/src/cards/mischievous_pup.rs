use cantrip_api::*;
use cantrip_mechanics::effects::return_first_target_to_hand;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flash is a core keyword. "Return up to one other target permanent you control to its owner's hand" (CR 400.3, 115.1): the Pup
/// itself is not a legal target, and a target that is no longer legal as the trigger resolves is not returned.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        return_first_target_to_hand,
        "When this creature enters, return up to one other target permanent you control to its owner's hand.",
    )
    .targets(&[up_to_one(targets::other_permanent_you_control_filter)])],
    ..oracle::MISCHIEVOUS_PUP
};
