use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Lifelink is a core keyword. "Put a +1/+1 counter on each of up to two other target creatures you control" (CR 115.1, 601.2c,
/// 122.1): one slot taking up to two targets, none of them the Savior; every target still legal as the ability resolves gets a
/// counter, together (CR 101.4).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::plus_one_counter_on_each_target,
        "When this creature enters, put a +1/+1 counter on each of up to two other target creatures you control.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL.between(0, 2)])],
    ..oracle::FELIDAR_SAVIOR
};
