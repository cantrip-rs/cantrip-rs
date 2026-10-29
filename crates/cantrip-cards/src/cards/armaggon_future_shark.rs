use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Destroy up to three target creatures" (CR 601.2c, 701.8): one slot of up to three targets; the ones still legal are
/// destroyed together.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::destroy_each_target, "When Armaggon enters, destroy up to three target creatures.")
        .targets(&[targets::CREATURE.between(0, 3)])],
    ..oracle::ARMAGGON_FUTURE_SHARK
};
