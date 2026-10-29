use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        effects::this_gets::<1, 0>,
        "Alliance \u{2014} Whenever another creature you control enters, this creature gets +1/+0 until end of turn.",
    )],
    ..oracle::EAST_WIND_AVATAR
};
