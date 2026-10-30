use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you face a dilemma, each opponent loses 2 life and you gain 2 life." (CR 603.2, 700.2, 119.3)
fn face_a_dilemma(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_loses_life_you_gain_life::<2>(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_FACE_A_DILEMMA,
        face_a_dilemma,
        "Whenever you face a dilemma, each opponent loses 2 life and you gain 2 life. (You face a dilemma as you choose one or more modes for a spell or ability.)",
    )],
    ..oracle::ELIM_GARAK_SIMPLE_TAILOR
};

