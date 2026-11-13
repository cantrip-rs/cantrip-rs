use cantrip_api::*;
use cantrip_mechanics::{choose, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        choose::look_at_top_cards_separate_into_face_down_and_face_up_piles_opponent_chooses::<5>,
        "When this creature enters, look at the top five cards of your library and separate them into a face-down pile and a face-up pile. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.",
    )],
    ..oracle::CURATOR_OF_DESTINIES
};
