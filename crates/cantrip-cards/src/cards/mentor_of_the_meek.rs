use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
        |g, item| effects::may_pay_to_draw(g, item, cost("{1}")),
        "Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.",
    )],
    ..oracle::MENTOR_OF_THE_MEEK
};
