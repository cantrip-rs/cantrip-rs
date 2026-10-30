use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::SPELL_YOU_CAST_DURING_AN_OPPONENTS_TURN,
        |g, item| effects::surveil(g, item, 2),
        "Whenever you cast a spell during an opponent's turn, surveil 2.",
    )],
    ..oracle::UNWELCOME_SPRITE
};
