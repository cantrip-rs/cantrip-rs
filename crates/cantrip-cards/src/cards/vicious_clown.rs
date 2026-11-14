use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
        effects::this_gets::<2, 0>,
        "Whenever another creature you control with power 2 or less enters, this creature gets +2/+0 until end of turn.",
    )],
    ..oracle::VICIOUS_CLOWN
};
