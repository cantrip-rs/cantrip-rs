use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ANOTHER_NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        effects::draw_a_card_and_lose_1_life,
        "Whenever another nontoken creature you control dies, you draw a card and lose 1 life.",
    )],
    ..oracle::AGENT_VENOM
};
