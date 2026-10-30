use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_FACE_A_DILEMMA, effects::each_opponent_loses_life_you_gain_life::<1>,
        "Whenever you face a dilemma, each opponent loses 1 life and you gain 1 life. (You face a dilemma as you choose one or more modes for a spell or ability.)")],
    ..oracle::OBSIDIAN_ORDER_OPERATIVE
};
