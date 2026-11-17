use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever another creature you control with power 2 or less enters, target creature you control gets +1/+1 until end of turn"
/// (CR 603.2, 115.1, 611.2a): the target is chosen when the ability goes on the stack, so it may be the Guardian, the creature
/// that entered or any other creature of yours.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
        effects::target_creature_gets::<1, 1>,
        "Whenever another creature you control with power 2 or less enters, target creature you control gets +1/+1 until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::NEIGHBORHOOD_GUARDIAN
};
