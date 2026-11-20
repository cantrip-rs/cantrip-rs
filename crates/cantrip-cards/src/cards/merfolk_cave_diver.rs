use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_EXPLORES,
        effects::this_gets_and_cant_be_blocked_this_turn::<1, 0>,
        "Whenever a creature you control explores, this creature gets +1/+0 until end of turn and can't be blocked this turn.",
    )],
    ..oracle::MERFOLK_CAVE_DIVER
};
