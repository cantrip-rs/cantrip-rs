use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_DRAW_YOUR_SECOND_CARD,
        effects::this_gets_and_cant_be_blocked_this_turn::<1, 2>,
        "Whenever you draw your second card each turn, this creature gets +1/+2 until end of turn and can't be blocked this turn.",
    )],
    ..oracle::OTTER_PENGUIN
};
