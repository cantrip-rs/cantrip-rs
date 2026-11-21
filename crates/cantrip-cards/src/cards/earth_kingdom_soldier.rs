use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Vigilance is a core keyword read from its characteristics. "When this creature enters, put a +1/+1 counter on each of up to
/// two target creatures you control" (CR 115.1, 122.1a): each target still legal as it resolves gets one.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::plus_one_counter_on_each_target,
        "When this creature enters, put a +1/+1 counter on each of up to two target creatures you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL.between(0, 2)])],
    ..oracle::EARTH_KINGDOM_SOLDIER
};
