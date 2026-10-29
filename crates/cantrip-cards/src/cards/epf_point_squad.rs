use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        effects::plus_one_counter_on_this,
        "Alliance \u{2014} Whenever another creature you control enters, put a +1/+1 counter on this creature.",
    )],
    ..oracle::EPF_POINT_SQUAD
};
