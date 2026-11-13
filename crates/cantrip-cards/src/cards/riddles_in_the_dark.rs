use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], choose::look_at_top_cards_separate_into_face_down_and_face_up_piles_opponent_chooses::<4>),
    ..oracle::RIDDLES_IN_THE_DARK
};
