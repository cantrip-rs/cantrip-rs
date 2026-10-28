use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// Flying is a core keyword. "When this creature enters, scry 2" (CR 701.22a): its controller looks at the top two cards of their
/// library, then puts any number of them on the bottom and the rest on top in any order.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::scry_two, "When this creature enters, scry 2.")],
    ..oracle::WAKANDAN_DRONE_FLOCK
};
