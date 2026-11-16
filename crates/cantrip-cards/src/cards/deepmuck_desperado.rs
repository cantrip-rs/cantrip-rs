use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::YOU_COMMIT_A_CRIME};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_COMMIT_A_CRIME,
        effects::each_opponent_mills::<3>,
        "Whenever you commit a crime, each opponent mills three cards. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )
    .triggers_each_turn(1)],
    ..oracle::DEEPMUCK_DESPERADO
};
