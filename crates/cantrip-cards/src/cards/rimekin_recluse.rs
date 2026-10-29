use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return up to one other target creature to its owner's hand.")
        .targets(&[up_to_one(targets::other_creature_filter)])],
    ..oracle::RIMEKIN_RECLUSE
};
