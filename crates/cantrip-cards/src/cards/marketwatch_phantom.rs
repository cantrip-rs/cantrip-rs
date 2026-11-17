use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever another creature you control with power 2 or less enters, this creature gains flying until end of turn" (CR 603.2,
/// 611.2a): the creature's power is judged as it enters, counters and static effects included (the 2024-02-02 ruling), so the
/// shared trigger reads it from the entering object.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
        effects::this_gains_flying,
        "Whenever another creature you control with power 2 or less enters, this creature gains flying until end of turn.",
    )],
    ..oracle::MARKETWATCH_PHANTOM
};
