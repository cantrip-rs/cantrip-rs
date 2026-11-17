use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever another creature you control with power 2 or less enters, surveil 1" (CR 603.6a, 701.25): the entering creature's
/// power is read as it enters, counters and static abilities included (the ruling on Mentor of the Meek), and this creature
/// entering does not trigger it.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
        |g, item| effects::surveil(g, item, 1),
        "Whenever another creature you control with power 2 or less enters, surveil 1.",
    )],
    ..oracle::SNARLING_GOREHOUND
};
