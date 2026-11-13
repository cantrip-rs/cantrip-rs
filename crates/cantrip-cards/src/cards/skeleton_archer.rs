use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // The target is chosen as the trigger goes on the stack (CR 603.3d); the Archer deals the damage (CR 120.3).
    abilities: &[triggered(triggers::THIS_ENTERS, effects::damage_first_target::<1>, "When this creature enters, it deals 1 damage to any target.").targets(&[targets::ANY])],
    ..oracle::SKELETON_ARCHER
};
