use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Trample is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        effects::this_gets::<1, 0>,
        "Whenever another creature you control enters, this creature gets +1/+0 until end of turn.",
    )],
    ..oracle::CROSSROADS_WATCHER
};
